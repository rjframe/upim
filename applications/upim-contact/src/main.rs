#![feature(int_error_matching)]
#![feature(iter_advance_by)]
#![feature(option_result_contains)]
#![feature(pattern)]
#![allow(dead_code)] // TODO: Only for early development

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

use anyhow::anyhow;

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
            );

            Proc::new("upim-edit")
                .args(&["-C", collection, &name])
                .spawn()?
                .wait()?;

            None
        },
        Command::Edit(_name) => {
            // TODO: match name
            todo!();
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
fn new_normalized_name(name: &str, def_collection_path: &Path) -> String {
    use std::path::PathBuf;

    // TODO: Replace all invalid filename characters for Windows, Mac, Linux
    let name = name.replace(' ', "_");
    let path = PathBuf::from(def_collection_path);

    let mut i = 0;
    loop {
        let mut filename = if i > 0 {
            let mut n = name.clone();
            n.push_str(&i.to_string());
            n
        } else {
            name.clone()
        };

        filename.push_str(".contact");

        if ! path.join(&filename).exists() {
            // TODO: Check fs::metadata for errors. Inability to access the
            // filesystem will be an infinite loop.
            break filename;
        } else {
            i+= 1;
        }
    }
}
