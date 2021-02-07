#![feature(bool_to_option)]
#![feature(drain_filter)]

// TODO - polish/finish/update/fix for man page:
//
// Usage: upim-edit [options] [file]
//
// -C <collection-name> <file-name> - Create/edit in the named collection.
// -o <path>                        - Save at the specified path rather than in
//                                    the collection folder
// --conf <path>                    - Use the following configuration file
//                                    instead of the system/user upim-edit.conf.
//
// Without -C or -o, will start a blank document and save in the current working
// directory.

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
// ---

// TODO: Instead of defaulting to vim, try the system's associated editor for
// the file type first?

use std::{
    path::PathBuf,
    env
};

use upim_core::config::*;

use anyhow::anyhow;

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

fn main() -> anyhow::Result<()> {
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
