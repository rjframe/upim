//! upim-edit - uPIM notes editor
//!
//! upim-edit wraps an external text editor and is designed for two primary
//! purposes:
//!
//! - scripting
//! - convenient editing and validation of notes in collections.

#![feature(bool_to_option)]
#![feature(drain_filter)]

// upim config file:
//
// ---
// ; if not present, only blank files are created.
// template_folder = /path/to/collections/template/folder
//
// [Collections]
// <name> = /path/to/collection/folder
// <name> = /path/to/collection/folder
// <name> = /path/to/collection/folder
// ---
//
// upim-edit config file:
//
// ---
// [DEFAULT]
//
// editor = command
// editor_arg = arg
//
// ; These override the others; most useful in CWD
// [Collections]
// <name> = /path/to/collection/folder
// <name> = /path/to/collection/folder
// <name> = /path/to/collection/folder
// ---

// TODO: Instead of defaulting to vim, try the system's associated editor for
// the file type first?

use std::{
    path::{Path, PathBuf},
    env
};

use upim_core::config::*;
use upim_note::Note;

use anyhow::anyhow;


// TODO: Return error, don't expect().
/// Launch the editor and wait for it to exit.
///
/// # Arguments
///
/// * editor - the editor's name.
/// * arg    - an option, if necessary, to tell the editor not to fork and
///            detach from the shell that starts it.
/// * path   - the path to a file to open, if desired.
fn launch_editor(editor: &str, arg: Option<&str>, path: Option<&str>) {
    // TODO: Open a temporary file, validate and move once closed?
    use std::process::Command;

    let mut args = vec![];

    if let Some(arg) = arg { args.push(arg); }
    if let Some(path) = path { args.push(path); }

    let mut child = Command::new(editor)
        .args(args)
        .spawn()
        .expect("Failed to execute command");

    let _res = child.wait().expect("Failed to wait");
    // TODO: Check res.
}

// TODO: Return codes.
fn main() -> anyhow::Result<()> {
    let options = Options::from_args(env::args());
    if options.is_err() {
        print_usage();
        return Ok(());
    }
    let options = options.unwrap();

    let conf = {
        let path = options.conf_path.or_else(find_default_configuration);

        if let Some(path) = path {
            read_config(path.to_str().unwrap())?
        } else {
            // TODO: If we can determine the editor from the environment we
            // currently don't need a configuration file.
            return Err(anyhow!("No configuration file found"));
        }
    };

    match options.action {
        Action::Edit => {
            // TODO: Finish implementation
            let editor = conf.get("editor").expect("No text editor set");
            let editor_arg = conf.get("editor_arg").map(|v| v.as_str());
            launch_editor(editor, editor_arg, None);
        },
        Action::AddTags(tags) => {
            let file = options.file.to_str().unwrap();
            let mut note = Note::read_from_file(file)?;

            for tag in &tags { note.insert_tag(tag); }
            note.write_to_file(file);
        },
        Action::AddAttribute(ref k, ref v) => {
            let file = options.file.to_str().unwrap();
            let mut note = Note::read_from_file(file)?;

            note.set_attribute(k, v);
            note.write_to_file(file);
        },
        Action::RemoveTags(tags) => {
            let file = options.file.to_str().unwrap();
            let mut note = Note::read_from_file(file)?;

            for tag in &tags { note.remove_tag(tag); }
            note.write_to_file(file);
        },
        Action::RemoveAttribute(ref k) => {
            let file = options.file.to_str().unwrap();
            let mut note = Note::read_from_file(file)?;

            note.remove_attribute(k);
            note.write_to_file(file);
        },
        Action::PrintTags => {
            let file = options.file.to_str().unwrap();
            let note = Note::read_from_file(file)?;

            for tag in note.tags().iter() {
                println!("{}", tag);
            }
        },
        Action::PrintAttributes => {
            let file = options.file.to_str().unwrap();
            let note = Note::read_from_file(file)?;

            for (k, v) in note.attributes().iter() {
                println!("{}:{}", k, v);
            }
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

#[derive(Debug, Eq, PartialEq)]
enum Action {
    Edit,
    AddTags(Vec<String>),
    AddAttribute(String, String),
    RemoveTags(Vec<String>),
    RemoveAttribute(String),
    PrintTags,
    PrintAttributes,
}

impl Default for Action {
    fn default() -> Action { Action::Edit }
}

#[derive(Debug, Default)]
struct Options {
    file: PathBuf,
    collection: Option<String>,
    conf_path: Option<PathBuf>,
    action: Action,
}

impl Options {
    fn from_args(args: env::Args) -> anyhow::Result<Self> {
        let args: Vec<String> = args.collect();
        Self::new(&args)
    }

    fn new(args: &[String]) -> anyhow::Result<Self> {
        let mut args = args;
        let mut opts = Self::default();

        if args.len() < 2 {
            return Err(anyhow!("Missing filename"));
        }
        args = &args[1..args.len()];

        // TODO: The majority of error messages assume the filename is given, so
        // could be incorrect. I'd need to fully scan the array on each error
        // and determine whether the final argument is a path to do better.
        while ! args.is_empty() {
            match args[0].as_ref() {
                "-C" => {
                    if args.len() < 2 {
                        return Err(anyhow!("Missing collection name"));
                    }
                    opts.collection = Some(args[1].clone());
                    args = &args[2..args.len()];
                },
                "--conf" => {
                    if args.len() < 2 {
                        return Err(anyhow!("Missing configuration path"));
                    }
                    if Path::new(&args[1]).exists() {
                        opts.conf_path = Some(PathBuf::from(&args[1]));
                        args = &args[2..args.len()];
                    } else {
                        return Err(anyhow!(
                            "The path {} does not exist", args[1]
                        ));
                    }
                },
                "--tags" => {
                    opts.action = Action::PrintTags;
                    args = &args[1..args.len()];
                },
                "--attributes" => {
                    opts.action = Action::PrintAttributes;
                    args = &args[1..args.len()];
                },
                "--add-tags" => {
                    let tags = read_tags(&args)?;
                    assert!(tags.len() < args.len());

                    args = &args[tags.len()+1..args.len()];
                    opts.action = Action::AddTags(tags);
                },
                "--add-attr" => {
                    if args.len() < 3 {
                        return Err(anyhow!("Missing attribute data"));
                    }

                    opts.action = Action::AddAttribute(
                        args[1].clone(),
                        args[2].clone(),
                    );
                    args = &args[3..args.len()];
                },
                "--remove-tags" => {
                    let tags = read_tags(&args)?;
                    assert!(tags.len() < args.len());

                    args = &args[tags.len()+1..args.len()];
                    opts.action = Action::RemoveTags(tags);
                },
                "--remove-attr" => {
                    if args.len() < 2 {
                        return Err(anyhow!("Missing attribute name"));
                    }

                    opts.action = Action::RemoveAttribute(args[1].clone());
                    args = &args[2..args.len()];
                },
                _ => {
                    opts.file = PathBuf::from(&args[0]);
                    if args.len() > 1 {
                        // TODO: Better error message.
                        return Err(
                            anyhow!("The path must be the last argument")
                        );
                    }
                    args = &args[1..args.len()];
                }
            }
        }

        if opts.is_valid() {
            Ok(opts)
        } else {
            // TODO: I should be able to be more descriptive.
            Err(anyhow!("Invalid input"))
        }
    }

    fn is_valid(&self) -> bool {
        self.file != PathBuf::default() &&
        if self.collection.is_some() {
            ! self.file.is_absolute()
        } else {
            true
        }
    }
}

fn read_tags(args: &[String]) -> anyhow::Result<Vec<String>> {
    let mut tags = vec![];
    let mut i = 1;

    while args[i].starts_with('@') {
        tags.push(args[i].to_string());
        i += 1;

        if i == args.len() {
            return Err(anyhow!("Missing file name"));
        }
    }

    if tags.is_empty() {
        return Err(anyhow!("No tags provided"));
    }

    Ok(tags)
}

/// Read the global uPIM and the upim-edit configurations.
///
/// # Arguments
///
/// * path - The path of the upim-edit configuration file.
fn read_config(path: &str) -> anyhow::Result<Config> {
    let mut conf = Config::read_from_file(path)?;

    if conf.get("editor").is_none() {
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

    if conf.get("editor_arg").is_none() {
        let editor = conf.get("editor").unwrap();

        // If we know what argument an editor needs to tell it to run in the
        // foreground, we add it here; otherwise assume nothing is necessary.
        if editor == "vim" {
            conf = conf.set_default("editor_arg", "-f");
        }
    }

    // TODO: Once I can inspect errors, do so.
    let global = read_upim_configuration().unwrap();

    if let Some(folder) = global.get("template_folder") {
        conf = conf.set_default("template_folder", folder);
    };

    for coll in global.variables_in_group("Collections").iter() {
        if conf.get_group("Collections", &coll).is_none() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_path() {
        let args = vec!["upim-edit".into(), "some-file.txt".into()];
        let opts = Options::new(&args).unwrap();
        assert_eq!(opts.file.to_str().unwrap(), "some-file.txt");
        assert_eq!(opts.action, Action::Edit);

        let args = vec!["upim-edit".into(), "/tmp/some-file.txt".into()];
        let opts = Options::new(&args).unwrap();
        assert_eq!(opts.file.to_str().unwrap(), "/tmp/some-file.txt");
        assert_eq!(opts.action, Action::Edit);
    }

    #[test]
    fn args_specify_collection() {
        let args = vec!["upim-edit".into(),
            "-C".into(), "coll".into(), "some-file.txt".into()
        ];

        let opts = Options::new(&args).unwrap();
        assert_eq!(opts.file.to_str().unwrap(), "some-file.txt");
        assert_eq!(opts.collection.unwrap(), "coll");
        assert_eq!(opts.action, Action::Edit);
    }

    #[test]
    fn args_with_collection_path_must_be_relative() {
        let args = vec!["upim-edit".into(),
            "-C".into(), "coll".into(), "/tmp/some-file.txt".into()
        ];

        let opts = Options::new(&args);
        assert!(opts.is_err());
    }

    // TODO: The --conf path must exist; need cross-platform test.
    #[cfg(unix)]
    #[test]
    fn args_conf_path() {
        let args = vec!["upim-edit".into(),
            "--conf".into(), "/dev/null".into(),
            "/tmp/some-file.txt".into()
        ];

        let opts = Options::new(&args).unwrap();
        assert_eq!(opts.conf_path.unwrap().to_str().unwrap(), "/dev/null");
        assert_eq!(opts.file.to_str().unwrap(), "/tmp/some-file.txt");
        assert_eq!(opts.action, Action::Edit);
    }

    #[test]
    fn args_tags() {
        let args = vec!["upim-edit".into(),
            "--tags".into(), "/tmp/some-file.txt".into()
        ];

        let opts = Options::new(&args).unwrap();
        assert_eq!(opts.file.to_str().unwrap(), "/tmp/some-file.txt");
        assert_eq!(opts.action, Action::PrintTags);
    }

    #[test]
    fn args_attributes() {
        let args = vec!["upim-edit".into(),
            "--attributes".into(), "/tmp/some-file.txt".into()
        ];

        let opts = Options::new(&args).unwrap();
        assert_eq!(opts.file.to_str().unwrap(), "/tmp/some-file.txt");
        assert_eq!(opts.action, Action::PrintAttributes);
    }

    #[test]
    fn args_add_tags() {
        let args = vec!["upim-edit".into(),
            "--add-tags".into(), "@tag1".into(), "@tag2".into(),
            "/tmp/some-file.txt".into()
        ];

        let tags = vec!["@tag1".into(), "@tag2".into()];

        let opts = Options::new(&args).unwrap();
        assert_eq!(opts.action, Action::AddTags(tags));
    }

    #[test]
    fn args_add_tags_missing_tags() {
        let args = vec!["upim-edit".into(),
            "--add-tags".into(), "/tmp/some-file.txt".into()
        ];

        assert!(Options::new(&args).is_err());
    }

    #[test]
    fn args_add_tags_invalid_tag_name() {
        let args = vec!["upim-edit".into(),
            "--add-tags".into(), "tag1".into(), "/tmp/some-file.txt".into()
        ];

        assert!(Options::new(&args).is_err());
    }

    #[test]
    fn args_add_attribute() {
        let args = vec!["upim-edit".into(),
            "--add-attr".into(), "key".into(), "value".into(),
            "/tmp/some-file.txt".into()
        ];

        let opts = Options::new(&args).unwrap();
        assert_eq!(
            opts.action,
            Action::AddAttribute("key".into(), "value".into())
        );
    }

    #[test]
    fn args_add_attribute_missing_one_kv() {
        let args = vec!["upim-edit".into(),
            "--add-attr".into(), "key".into(),
            "/tmp/some-file.txt".into()
        ];

        assert!(Options::new(&args).is_err());
    }

    #[test]
    fn args_add_attribute_missing_both_kv() {
        let args = vec!["upim-edit".into(),
            "--add-attr".into(), "/tmp/some-file.txt".into()
        ];

        assert!(Options::new(&args).is_err());
    }

    #[test]
    fn args_add_attribute_spaces_in_kv() {
        let args = vec!["upim-edit".into(),
            "--add-attr".into(), "my key".into(), "my value".into(),
            "/tmp/some-file.txt".into()
        ];

        let opts = Options::new(&args).unwrap();
        assert_eq!(
            opts.action,
            Action::AddAttribute("my key".into(), "my value".into())
        );
    }

    #[test]
    fn args_remove_tags() {
        let args = vec!["upim-edit".into(),
            "--remove-tags".into(), "@tag1".into(), "@tag2".into(),
            "/tmp/some-file.txt".into()
        ];

        let tags = vec!["@tag1".into(), "@tag2".into()];

        let opts = Options::new(&args).unwrap();
        assert_eq!(opts.action, Action::RemoveTags(tags));
    }

    #[test]
    fn args_remove_tags_missing_tags() {
        let args = vec!["upim-edit".into(),
            "--remove-tags".into(), "/tmp/some-file.txt".into()
        ];

        assert!(Options::new(&args).is_err());
    }

    #[test]
    fn args_remove_tags_invalid_tag_name() {
        let args = vec!["upim-edit".into(),
            "--remove-tags".into(), "tag1".into(), "/tmp/some-file.txt".into()
        ];

        assert!(Options::new(&args).is_err());
    }

    #[test]
    fn args_remove_attribute() {
        let args = vec!["upim-edit".into(),
            "--remove-attr".into(), "key".into(), "/tmp/some-file.txt".into()
        ];

        let opts = Options::new(&args).unwrap();
        assert_eq!(opts.action, Action::RemoveAttribute("key".into()));
    }

    #[test]
    fn args_remove_attribute_no_name() {
        let args = vec!["upim-edit".into(),
            "--remove-attr".into(), "/tmp/some-file.txt".into()
        ];

        assert!(Options::new(&args).is_err());
    }
}
