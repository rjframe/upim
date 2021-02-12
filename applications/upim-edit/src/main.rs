//! upim-edit - uPIM notes editor
//!
//! upim-edit wraps an external text editor and is designed for two primary
//! purposes:
//!
//! - scripting
//! - convenient editing and validation of notes in collections.

#![feature(bool_to_option)]
#![feature(drain_filter)]

// TODO: Instead of defaulting to vim, try the system's associated editor for
// the file type first?

mod args;

use std::{
    path::{Path, PathBuf},
    env,
    fs,
};

use upim_core::config::*;
use upim_note::Note;

use crate::{
    args::*,
};

use anyhow::anyhow;


fn main() -> anyhow::Result<()> {
    let options = Options::from_args(env::args());
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
            read_config(&path)?
        } else {
            // TODO: If we can determine the editor from the environment we
            // currently don't need a configuration file.
            return Err(anyhow!("No configuration file found"));
        }
    };

    match options.action {
        Action::Edit => {
            let editor = conf.get_default("editor").expect("No text editor set");
            let editor_arg = conf.get_default("editor_arg").map(|v| v.as_str());
            let (path, templ) = determine_file_path(&options, &conf)?;

            if let Some(templ) = templ {
                fs::copy(templ, &path)?;
            };

            launch_editor(editor, editor_arg, &path)?;
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

            for (k, v) in note.attributes().iter() {
                println!("{}:{}", k, v);
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
        "\t--content                 - print the note's content then exit\n",
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
fn launch_editor(editor: &str, arg: Option<&str>, path: &PathBuf)
-> anyhow::Result<()>
{
    use std::{
        process::Command,
        time::SystemTime,
        io::{self, Write},
    };

    let mut args = vec![];
    if let Some(arg) = arg { args.push(arg); }
    // TODO: Should probably fail on invalid UTF-8.
    let p = path.to_string_lossy();
    args.push(&p);

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

    // TODO: Check exit code here?
    Command::new(editor)
        .args(args)
        .spawn()?
        .wait()?;

    let maybe_modified = if path.exists() {
        fs::metadata(&path)?.modified().unwrap_or_else(|_| SystemTime::now())
    } else {
        SystemTime::now()
    };

    // We assume the note was valid when opened, so we only need to perform
    // validation if it's been modified. We only validate the header -- we
    // assume the document is properly-encoded UTF-8.
    if maybe_modified != last_modified {
        if let Err(e) = Note::validate_header(&path) {
            println!("Error validating note. {}", e);
            io::stdout().write_all(
                b"Would you like to re-open the file to fix? [Y/n] "
            )?;
            io::stdout().flush()?;

            let mut inp = String::new();
            io::stdin().read_line(&mut inp)?;

            match inp.trim() {
                "" | "y" | "Y" => launch_editor(editor, arg, path),
                _ => Err(e.into()),
            }
        } else {
            Ok(())
        }
    } else {
        // File wasn't saved. Do nothing.
        Ok(())
    }
}

/// Read the global uPIM and the upim-edit configurations.
///
/// # Arguments
///
/// * path - The path of the upim-edit configuration file.
fn read_config(path: &Path) -> anyhow::Result<Config> {
    let mut conf = Config::read_from_file(path)?;

    if conf.get_default("editor").is_none() {
        let editor = env::var_os("EDITOR").map(|e| e.into_string());

        if let Some(editor) = editor {
            if let Ok(editor) = editor {
                conf = conf.set_default("editor", &editor);
            } else {
                return Err(anyhow!("Cannot convert $EDITOR to a UTF-8 string"));
            }
        } else {
            conf = conf
                .set_default("editor", "vim")
                .set_default("editor_arg", "-f");
        }
    }

    if conf.get_default("editor_arg").is_none() {
        let editor = conf.get_default("editor").unwrap();

        // If we know what argument an editor needs to tell it to run in the
        // foreground, we add it here; otherwise assume nothing is necessary.
        if editor == "vim" {
            conf = conf.set_default("editor_arg", "-f");
        }
    }

    // TODO: Once I can inspect errors, do so.
    let global = read_upim_configuration().unwrap();

    if conf.get_default("template_folder").is_none() {
        if let Some(folder) = global.get_default("template_folder") {
            conf = conf.set_default("template_folder", folder);
        };
    }

    for coll in global.variables_in_group("Collections").iter() {
        if conf.get("Collections", &coll).is_none() {
            conf = conf.set(
                "Collections",
                &coll,
                global[("Collections", coll.as_str())].as_str()
            );
        }
    }

    Ok(conf)
}

/// Get the path to the first upim-edit.conf file found (upim-edit.ini on
/// Windows).
fn find_default_configuration() -> Option<PathBuf> {
    let filename = if cfg!(windows) {
        "upim-edit.ini"
    } else {
        "upim-edit.conf"
    };

    let mut paths = get_upim_configuration_dirs().unwrap_or_default();

    paths.iter_mut()
        .find_map(|p| {
            p.push(filename);
            p.exists().then_some(p.clone())
        })
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
            anyhow!("Cannot specify an absolute path with a collection"));
    }

    if let Some(path) = conf.get("Collections", &coll) {
        // We need to use the collection directory to generate the file's
        // absolute path.

        let mut path = PathBuf::from(path);
        path.push(&options.file);

        if path.exists() {
            Ok((path, None))
        } else {
            // We're creating the file, so need the path to the template if one
            // exists.

            if let Some(templ) = conf.get_default("template_folder") {
                let mut templ = PathBuf::from(templ);
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
