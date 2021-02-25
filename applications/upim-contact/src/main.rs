#![feature(option_result_contains)]
#![feature(pattern)]
#![feature(str_split_once)]
#![allow(dead_code)] // TODO: Only for early development

mod args;
mod contact;
mod either;
mod filter;

use std::{
    path::PathBuf,
    str::FromStr as _,
    env,
};

use upim_core::config::Config;

use args::{Command, Options};
use filter::Condition;

use anyhow::anyhow;


fn main() -> anyhow::Result<()> {
    let opts = Options::new(env::args())?;
    let conf = read_config(opts.conf_path)?;

    let search = match opts.cmd_or_alias {
        Command::Search => opts.filter,
        Command::Alias(ref name) => {
            match conf.get("Aliases", name) {
                Some(alias) => {
                    let cond1 = Condition::from_str(alias)?;

                    opts.filter.map(|cond2| {
                        Condition::And(Box::new((cond1, cond2)))
                    })
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

    if let Some(_search) = search {
        // TODO: Perform the search.
    };

    Ok(())
}

fn read_config(path: Option<PathBuf>) -> anyhow::Result<Config> {
    todo!();
}
