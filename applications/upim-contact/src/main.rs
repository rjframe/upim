#![feature(assoc_char_funcs)]
#![feature(int_error_matching)]
#![feature(iter_advance_by)]
#![feature(option_result_contains)]
#![feature(pattern)]
#![feature(str_split_once)]
#![allow(dead_code)] // TODO: Only for early development

mod args;
mod config;
mod contact;
mod either;
mod filter;

use std::{
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
        Command::New(_name) => {
            todo!();
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
