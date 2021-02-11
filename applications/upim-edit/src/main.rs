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

mod args;

use std::{
    path::PathBuf,
    env,
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
            let editor = conf.get_default("editor").expect("No text editor set");
            let editor_arg = conf.get_default("editor_arg").map(|v| v.as_str());
            launch_editor(editor, editor_arg, &options.file)?;
        },
        Action::AddTags(tags) => {
            let file = options.file.to_str().unwrap();
            let mut note = Note::read_from_file(file)?;

            for tag in &tags { note.insert_tag(tag); }
            note.write_to_file(file)?;
        },
        Action::AddAttribute(ref k, ref v) => {
            let file = options.file.to_str().unwrap();
            let mut note = Note::read_from_file(file)?;

            note.set_attribute(k, v);
            note.write_to_file(file)?;
        },
        Action::RemoveTags(tags) => {
            let file = options.file.to_str().unwrap();
            let mut note = Note::read_from_file(file)?;

            for tag in &tags { note.remove_tag(tag); }
            note.write_to_file(file)?;
        },
        Action::RemoveAttribute(ref k) => {
            let file = options.file.to_str().unwrap();
            let mut note = Note::read_from_file(file)?;

            note.remove_attribute(k);
            note.write_to_file(file)?;
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
        fs,
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

    // See if we need to validate the note. We assume that it was valid when it
    // was opened, so only need to check it if it's been modified. We also
    // assume that the note content is valid UTF-8. TODO: Check that?
    if maybe_modified != last_modified {
        let res = Note::validate_header(&path);
        if let Err(e) = res {
            // TODO: Offer to re-open to fix.
            Err(e)?
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
fn read_config(path: &str) -> anyhow::Result<Config> {
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

    if let Some(folder) = global.get_default("template_folder") {
        conf = conf.set_default("template_folder", folder);
    };

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
