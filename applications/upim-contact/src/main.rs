#![feature(int_error_matching)]
#![feature(iter_advance_by)]
#![feature(option_result_contains)]
#![feature(pattern)]

mod args;
mod config;
mod contact;
mod either;
mod filter;

use std::{
    path::Path,
    str::FromStr as _,
    env,
};

use anyhow::{anyhow, Context};

use upim_core::paths::collection_path;

use args::{Command, Options, substitute_alias};
use config::*;
use contact::{read_contacts, print_contacts};
use filter::Query;


fn main() -> anyhow::Result<()> {
    use std::process::Command as Proc;

    let opts = Options::new(env::args())?;

    let conf = read_config(opts.conf_path)
        .map_err(|errs| {
            for e in errs {
                eprintln!("Error: {}", e);
            }
            return anyhow!("Failed to read configuration file.");
        })?;

    let search = match opts.cmd_or_alias {
        Command::Search => opts.filter,
        Command::Alias(ref name) => {
            match conf.get("Aliases", name) {
                Some(alias) => {
                    let alias = alias.strip_prefix("--filter")
                        .unwrap_or(alias)
                        .trim_start();

                    let alias = match opts.alias_params {
                        Some(ref p) => {
                            let (len, a) = substitute_alias(p, alias)?;

                            if len != p.len() {
                                return Err(anyhow!(
                                    "Expected {} parameters, but received {}: \
                                    {:?}",
                                    len,
                                    p.len(),
                                    p
                                ));
                            }

                            a
                        },
                        None => {
                            // Check for missing values for parameter
                            // substitutions. We get back a cloned alias, which
                            // we'd need to do anyway.
                            let (_, alias) = substitute_alias(&[], alias)?;
                            alias
                        },
                    };

                    let alias = Query::from_str(&alias)?;

                    opts.filter.or(Some(alias))
                },
                None => return Err(anyhow!("Unknown alias: {}", name)),
            }
        },
        Command::New(name) => {
            let collection = if let Some(coll) = &opts.collection {
                coll
            } else {
                &conf["default_collection"]
            };

            let name = new_normalized_name(
                &name,
                &collection_path(&conf, &collection)?
            ).context("Cannot create new file")?;

            Proc::new("upim-edit")
                .args(&["-C", collection, &name])
                .spawn()?
                .wait()?;

            None
        },
        Command::Edit(name) => {
            let collection = if let Some(coll) = &opts.collection {
                coll
            } else {
                &conf["default_collection"]
            };

            if name.is_left() {
                let name = name.left().unwrap();
                let name = normalized_name(
                    &name,
                    &collection_path(&conf, &collection)?
                )?;

                Proc::new("upim-edit")
                    .args(&["-C", collection, &name])
                    .spawn()?
                    .wait()?;
            } else {
                let path = name.right().unwrap();
                let path = if path.is_relative() {
                    Path::new(collection).join(path)
                } else {
                    path.to_owned()
                };

                Proc::new("upim-edit")
                    .args(&[path.to_str().unwrap()])
                    .spawn()?
                    .wait()?;
            }

            None
        },
    };

    if let Some(search) = search {
        let collection = opts.collection
            .unwrap_or_else(|| conf["default_collection"].to_owned());
        let path = collection_path(&conf, &collection)?;
        let sep = &conf["field_separator"];

        let contacts = read_contacts(&path, search.condition)?;
        print_contacts(&contacts, &search.select, sep);
    };

    Ok(())
}

/// Return the filename for a new contact.
fn new_normalized_name(name: &str, def_collection_path: &Path)
-> anyhow::Result<String> {
    use std::path::PathBuf;

    let path = PathBuf::from(def_collection_path);

    if ! path.exists() {
        return Err(anyhow!("Cannot access collection directory"))
    }

    let filename = normalize_contact_name(name);

    let mut i = 0;
    loop {
        let filename = add_name_index_and_ext(&filename, i);

        if ! path.join(&filename).exists() {
            // An inability to access the filesystem between the path check
            // above and this filename check will be an infinite loop. We check
            // the directory path again to prevent that.
            if ! path.exists() {
                return Err(anyhow!("Cannot access collection directory"))
            }

            break Ok(filename);
        } else {
            i += 1;
        }
    }
}

fn normalized_name(name: &str, def_collection_path: &Path)
-> anyhow::Result<String> {
    use std::fs::read_dir;

    let name = normalize_contact_name(name);

    let mut files = read_dir(def_collection_path)?
        .map(|r| r.map(|e| e.path())).filter_map(|r| r.ok())
        .filter(|p| p.is_file())
        // TODO: Verify this unwrap is safe - I'm about 95% sure.
        .map(|p| p.file_stem().unwrap().to_string_lossy().into_owned())
        .filter(|f| f.starts_with(&name))
        .collect::<Vec<String>>();

    if ! files.is_empty() {
        files.sort();
        let f = &mut files[0];
        f.push_str(".contact");
        Ok(f.to_owned())
    } else {
        Err(anyhow!("Contact '{}' not found", name))
    }
}

fn normalize_contact_name(name: &str) -> String {
    // TODO: Replace all invalid filename characters for Windows, Mac, Linux
    name.replace(' ', "_")
}

fn add_name_index_and_ext(name: &str, idx: u32) -> String {
    let mut name = name.to_owned();
    if idx > 0 {
        name.push_str(&idx.to_string());
    }
    name.push_str(".contact");
    name
}
