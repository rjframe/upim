//! upim-edit - uPIM notes editor
//!
//! upim-edit wraps an external text editor and is designed for two primary
//! purposes:
//!
//! - scripting
//! - convenient editing and validation of notes in collections.

#![feature(bool_to_option)]
#![feature(drain_filter)]

mod args;

use std::{
    path::{Path, PathBuf},
    env,
    fmt,
    fs,
};

use anyhow::{anyhow, Context as _};

use upim_core::{
    config::*,
    error::FileError,
    paths::expand_tilde,
};
use upim_note::Note;

use crate::args::*;


fn main() -> anyhow::Result<()> {
    let options = Options::new(env::args());
    let options = if let Ok(opt) = options {
        opt
    } else {
        print_usage();
        return Ok(());
    };

    if options.action == Action::PrintHelp {
        print_usage();
        return Ok(());
    }

    let conf = {
        let path = options.conf_path.clone() // Clone to avoid partial move.
            .or_else(find_default_configuration);

        if let Some(path) = path {
            match read_config(&path) {
                Ok(conf) => conf,
                Err(errors) => {
                    for e in errors.iter() {
                        eprintln!("Error: {}", e);
                    }
                    return Err(anyhow!("Failed to read configuration file."));
                },
            }
        } else {
            // If we can determine the editor from the environment later we
            // don't need a configuration file.
            Config::default()
        }
    };

    match options.action {
        Action::Edit => {
            let editor = conf.get_default("editor")
                .ok_or_else(|| anyhow!("No text editor configured"))?;
            let editor_arg = conf.get_default("editor_arg").map(|v| v.as_str());

            let (path, templ) = determine_file_path(&options, &conf)?;

            if let Some(ref templ) = templ {
                fs::copy(templ, &path)
                    .context("While copying template")?;
            };

            launch_editor(editor, editor_arg, &path, templ.as_deref())?;
        },
        Action::AddTags(tags) => {
            let mut note = Note::read_from_file(&options.file)?;

            for tag in &tags { note.insert_tag(tag); }
            note.write_to_file(&options.file)?;
        },
        Action::AddAttribute(ref k, ref v) => {
            let mut note = Note::read_from_file(&options.file)?;

            note.set_attribute(k, v);
            note.write_to_file(&options.file)?;
        },
        Action::RemoveTags(tags) => {
            let mut note = Note::read_from_file(&options.file)?;

            for tag in &tags { note.remove_tag(tag); }
            note.write_to_file(&options.file)?;
        },
        Action::RemoveAttribute(ref k) => {
            let mut note = Note::read_from_file(&options.file)?;

            note.remove_attribute(k);
            note.write_to_file(&options.file)?;
        },
        Action::PrintTags => {
            let note = Note::read_header(&options.file)?;

            for tag in note.tags().iter() {
                println!("{}", tag);
            }
        },
        Action::PrintAttributes => {
            let note = Note::read_header(&options.file)?;

            for (k, v) in note.attributes() {
                println!("{}:{}", k, v);
            }
        },
        Action::PrintCollections => {
            for coll in conf.variables_in_group("Collections") {
                println!("{}", coll);
            }
        },
        Action::PrintContent => {
            let note = Note::read_from_file(&options.file)?;
            println!("{}", note.content());
        },
        Action::PrintHelp => {
            // we printed above, prior to reading the configuration file.
            panic!();
        },
    }

    Ok(())
}

fn print_usage() {
    println!(concat!(
        "Usage: upim-edit [options...] <file>\n",
        "Create and edit uPIM notes.\n\n",

        "\t-C <name>                 - Create/edit a note in the named ",
        "collection\n",
        "\t--conf <path>             - Use the specified configuration file\n",
        "\t--tags                    - Print the note's tags then exit\n",
        "\t--attributes              - Print the note's attributes then exit\n",
        "\t--collections             - Print the collections then exit\n",
        "\t--content                 - Print the note's content then exit\n",
        "\t--add-tags <tag>...       - Add one or more tags to the note\n",
        "\t--add-attr <name> <value> - Add or edit an attribute\n",
        "\t--remove-tags <tag>...    - Remove one or more tags from the note\n",
        "\t--remove-attr <name>      - Remove an attribute from the note\n",
        "\t--help                    - Print this help message\n",

        "\nWith the -C flag, <file> must be a path relative to the collection ",
        "folder.\nOtherwise it may be an absolute path or a path relative to ",
        "the current directory.\n\n",

        "A tag is an arbitrary group of text (except spaces) prefixed by '@'.",
        "\n\n",

        "Attributes are key-value pairs of text. Spaces are allowed in both ",
        "parts.\n`--add-attr` for an attribute that already exists will ",
        "replace its value with\nthe new value.\n",
    ));
}

/// Launch the editor and wait for it to exit.
///
/// # Arguments
///
/// * editor - the editor's name.
/// * arg    - an option, if necessary, to tell the editor not to fork and
///            detach from the shell that starts it.
/// * path   - the path to a file to create or edit.
/// * templ  - the path to the newly-created template file if applicable
fn launch_editor(
    editor: &str,
    arg: Option<&str>,
    path: &Path,
    templ: Option<&Path>
) -> anyhow::Result<()> {
    use std::{
        process::Command,
        time::SystemTime,
        io::{self, Write},
    };

    let mut args = vec![];
    if let Some(arg) = arg { args.push(arg); }
    args.push(path.to_str().ok_or_else(|| anyhow!("Invalid path"))?);

    // If we cannot read the file's last modification time, we call it `now`;
    // we'll do the same later, effectively treating the file as always
    // modified and will always validate it.
    //
    // We do return an error on permissions problems though -- lack of
    // permission to read metadata probably means we won't be able to write to
    // the file either. This may cause an unnecessary failure for systems that
    // set privileges for applications rather than (or in addition to) users,
    // since the editor might still have been able to edit the file.
    let last_modified = if path.exists() {
        fs::metadata(&path)?.modified().unwrap_or_else(|_| SystemTime::now())
    } else {
        SystemTime::now()
    };

    Command::new(editor)
        .args(args)
        .spawn()?
        .wait()?;

    let was_not_modified = if path.exists() {
        fs::metadata(&path)?.modified().unwrap_or_else(|_| SystemTime::now())
    } else {
        SystemTime::now()
    } == last_modified;

    // We assume the note was valid when opened, so we only need to perform
    // validation if it's been modified. We only validate the header -- we
    // assume the document is properly-encoded UTF-8.
    if was_not_modified {
        // If we just created the file from a template but the user did not
        // modify it, we remove the file. We never remove a file in the
        // templates directory.
        if let Some(templ) = templ {
            if path.parent() != templ.parent() {
                // This can only happen if someone creates a collection pointing
                // to it.
                fs::remove_file(path)?
            }
        }

        Ok(())
    } else {
        match Note::validate_header(&path) {
            Ok(()) => Ok(()),
            Err(e) => {
                println!("Error validating note. {}", e);
                print!("Would you like to re-open the file to fix? [Y/n] ");
                io::stdout().flush()?;

                let mut inp = String::new();
                io::stdin().read_line(&mut inp)?;

                match inp.trim() {
                    "" | "y" | "Y" => launch_editor(editor, arg, path, None),
                    _ => Err(e.into()),
                }
            },
        }
    }
}

/// Errors that can occur while reading information from our exteral
/// environment.
#[derive(Debug, Clone)]
enum ConfigurationError {
    Config(FileError),
    Environment(String),
}

impl fmt::Display for ConfigurationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ConfigurationError::Config(ref e) => e.fmt(f),
            ConfigurationError::Environment(ref s) => write!(f, "{}", s),
        }
    }
}

impl std::error::Error for ConfigurationError {}

impl From<FileError> for ConfigurationError {
    fn from(err: FileError) -> ConfigurationError {
        ConfigurationError::Config(err)
    }
}

/// Read the global uPIM and the upim-edit configurations.
///
/// # Arguments
///
/// * path - The path of the upim-edit configuration file.
fn read_config(path: &Path)
-> std::result::Result<Config, Vec<ConfigurationError>> {
    let mut conf = Config::read_from_file(path)
        .map_err(|v| v.iter()
            .map(|e| ConfigurationError::Config(e.clone()))
                .collect::<Vec<ConfigurationError>>())?;

    let mut errs = vec![];

    if conf.get_default("editor").is_none() {
        let editor = env::var_os("EDITOR").map(|e| e.into_string());

        if let Some(editor) = editor {
            if let Ok(editor) = editor {
                conf = conf.set_default("editor", &editor);
            } else {
                errs.push(
                    ConfigurationError::Environment(
                        "Cannot convert $EDITOR to a UTF-8 string".into()
                    )
                );
            }
        } else {
            errs.push(
                ConfigurationError::Environment(
                    "No text editor is configured".into()
                )
            );
        }
    }

    if conf.get_default("editor_arg").is_none() {
        // Safe to unwrap: we added editor above if it was missing.
        let editor = conf.get_default("editor").unwrap();

        // If we know what argument an editor needs to tell it to run in the
        // foreground, we add it here; otherwise assume nothing is necessary.
        if editor == "vim" || editor == "gvim" {
            conf = conf.set_default("editor_arg", "-f");
        }
        // Editors that require nothing:
        // - emacs (daemon mode not tested)
        // - nvim (headless mode not tested)
    }

    let global = read_upim_configuration()
        .map_err(|v| v.iter()
            .map(|e| ConfigurationError::Config(e.clone()))
                .collect::<Vec<ConfigurationError>>());

    let global = match global {
        Ok(c) => c,
        Err(mut e) => { errs.append(&mut e); return Err(errs); },
    };

    if conf.get_default("template_folder").is_none() {
        let folder = global.get_default("template_folder")
            .map(|f| expand_tilde(Path::new(f)))
            .flatten();

        if let Some(folder) = folder {
            conf = conf.set_default(
                "template_folder",
                &folder.to_string_lossy()
            );
        };
    }

    if conf.get_default("collection_base").is_none() {
        let folder = global.get_default("collection_base")
            .map(|f| expand_tilde(Path::new(f)))
            .flatten();

        if let Some(folder) = folder {
            conf = conf.set_default(
                "collection_base",
                &folder.to_string_lossy()
            );
        };
    }

    for coll in global.variables_in_group("Collections") {
        let path = global[("Collections", coll.as_str())].as_str();

        if let Some(path) = expand_tilde(Path::new(path)) {
            if conf.get("Collections", &coll).is_none() {
                conf = conf.set(
                    "Collections",
                    &coll,
                    &path.to_string_lossy()
                );
            }
        } else {
            errs.push(
                ConfigurationError::Environment(
                    "Cannot expand user's home directory".into()
                )
            );
        };
    }

    if errs.is_empty() {
        Ok(conf)
    } else {
        Err(errs)
    }
}

/// Get the path to the first upim-edit.conf file found.
fn find_default_configuration() -> Option<PathBuf> {
    find_application_configuration("upim-edit")
}

/// Determine the path of the file to create or edit based on the specified
/// collection.
///
/// # Returns
///
/// The path to the file to open, and if it is to be created from a template,
/// the path of the template to copy.
///
/// Returns an error if a collection is specified but an absolute path was given
/// on the command-line.
fn determine_file_path(options: &Options, conf: &Config)
-> anyhow::Result<(PathBuf, Option<PathBuf>)> {
    let coll = match &options.collection {
        Some(c) => c,
        None => { return Ok((options.file.clone(), None)); },
    };

    if options.file.is_absolute() {
        return Err(
            anyhow!("Cannot specify both an absolute path and a collection"));
    }

    if let Some(path) = conf.get("Collections", &coll) {
        // We need to use the collection directory to generate the file's
        // absolute path.

        let mut path = expand_tilde(Path::new(path))
            .ok_or_else(|| anyhow!("Cannot expand user's home"))?;

        path.push(&options.file);

        let path = if path.is_relative() {
            match conf.get_default("collection_base") {
                Some(base) => {
                    let mut base = expand_tilde(Path::new(base))
                        .ok_or_else(|| anyhow!("Cannot expand user's home"))?;
                    base.push(path);
                    base
                },
                None => {
                    return Err(anyhow!(
                        "Relative collection paths are not supported if \
                        collection_base is unset"
                    ));
                },
            }
        } else {
            path
        };

        if path.exists() {
            Ok((path, None))
        } else {
            // We're creating the file, so need the path to the template if one
            // exists.

            if let Some(templ) = conf.get_default("template_folder") {
                let mut templ = expand_tilde(Path::new(templ))
                    .ok_or_else(|| anyhow!("Cannot expand user home"))?;
                templ.push(&coll);
                templ.set_extension("template");

                if templ.exists() {
                    Ok((path, Some(templ)))
                } else {
                    Ok((path, None))
                }
            } else {
                Ok((path, None))
            }
        }
    } else {
        // The collection is not defined.
        Err(anyhow!("Unknown collection - {}", coll))
    }
}
