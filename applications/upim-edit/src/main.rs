#![feature(bool_to_option)]
#![feature(drain_filter)]

// TODO - polish/finish/update/fix for man page:
//
// Usage: upim-edit [options] <file>
//
// -C <collection-name>             - Create/edit in the named collection.
// --conf <path>                    - Use the following configuration file
//                                    instead of the system/user upim-edit.conf.
// --tags                           - instead of opening the file for editing,
//                                    print the list of tags.
// --attributes                     - instead of opening the file for editing,
//                                    print the key-value attributes.
//
// Without -C or -o, will start a blank document and save in the current working
// directory unless [file] is an absolute path.

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
// ; TODO: I haven't implemented this.
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

    let conf = if let Some(path) = find_default_configuration() {
        read_config(path.to_str().unwrap())?
    } else {
        // TODO: If we can determine the editor from the environment we
        // currently don't need a configuration file.
        return Err(anyhow!("No configuration file found"));
    };

    let editor = conf.get("editor").expect("No text editor set");
    let editor_arg = conf.get("editor_arg").map(|v| v.as_str());

    launch_editor(editor, editor_arg, None);

    Ok(())
}

fn print_usage() {
    println!("Usage: upim-edit [options...] <file>");
    println!("Create and edit uPIM notes.\n");

    println!("\t-C <collection-name> - Create/edit in the named collection");
    println!("\t--conf <path>        - Use the specified configuration file");
    println!("\t--tags               - Print the file's tags then exit");
    print!  ("\t--attributes         - Print the file's key-value attributes ");
    println!("then exit");
    println!("\t--help               - Print this help message");

    print!("\nWith the -C flag, <file> must be a relative path. Otherwise it ");
    println!("be an absolute or \nrelative path.");
}

#[derive(Debug, Eq, PartialEq)]
enum Action {
    Edit,
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
                        return Err(anyhow!("Missing collection name"));
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
        conf = conf.set(
            "Collections",
            &coll,
            global[("Collections", coll.as_str())].as_str()
        );
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
}
