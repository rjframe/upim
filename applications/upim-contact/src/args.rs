//! Command-line argument parsing

use std::{
    path::{Path, PathBuf},
    str::FromStr as _,
};

use anyhow::anyhow;

use crate::filter::Condition;

#[derive(Debug, Default)]
pub struct Options {
    // A command or filter alias.
    pub cmd_or_alias: String,
    // The non-default collection to use.
    pub collection: Option<String>,
    // An alternate configuration file.
    pub conf_path: Option<PathBuf>,
    // Multiple conditions are ANDed together and stored as one.
    // If there is no cmd_or_alias, there must be a filter.
    pub filter: Option<Condition>,
}

impl Options {
    pub fn new<T>(args: T) -> anyhow::Result<Self>
        where T: Iterator<Item = String>,
    {
        let args = &mut args.collect::<Vec<String>>();
        let mut args = &mut args[1..];
        let mut opts = Options::default();

        while ! args.is_empty() {
            match args[0].as_ref() {
                "-C" => {
                    if args.len() < 2 {
                        return Err(anyhow!("Missing collection name"));
                    }
                    opts.collection = Some(args[1].to_owned());
                    args = &mut args[2..];
                },
                "--conf" => {
                    if args.len() < 2 {
                        return Err(anyhow!("Missing configuration path"));
                    }
                    if Path::new(&args[1]).exists() {
                        opts.conf_path = Some(PathBuf::from(&args[1]));
                        args = &mut args[2..];
                    } else {
                        return Err(anyhow!(
                            "The path {} does not exist", args[1]
                        ));
                    }
                },
                "--filter" => {
                    if args.len() < 2 {
                        return Err(anyhow!("No query filter provided"));
                    }

                    let filter = match opts.filter {
                        Some(f) => {
                            Some(Condition::And(Box::new((
                                f,
                                Condition::from_str(&args[1])?
                            ))))
                        },
                        None => Some(Condition::from_str(&args[1])?)
                    };
                    opts.filter = filter;
                    args = &mut args[2..];
                },
                _ => {
                    // TODO: We need the list of aliases from the configuration.
                    // Then we'll build the relevant filters.
                    panic!();
                },
            }
        }

        Ok(opts)
    }
}

impl Options {
    /// Determine whether this is a valid [Options] object.
    ///
    /// If no command or alias was provided, then there must be a filter.
    /// If there is a command/alias, a filter is optional.
    #[allow(clippy::len_zero)]
    fn is_valid(&self) -> bool {
        self.cmd_or_alias.len() > 0 || self.filter.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::filter::FilterOp;

    #[test]
    fn args_collection() {
        let args = vec!["upim-contact", "-C", "work-contacts"];
        let args = args.iter().map(|s| s.to_string());

        let opts = Options::new(args).unwrap();
        assert!(! opts.is_valid());
        assert_eq!(opts.collection, Some("work-contacts".into()));
    }

    // TODO: The --conf path must exist; need cross-platform test.
    #[cfg(unix)]
    #[test]
    fn args_conf_path() {
        let args = vec!["upim-contact", "--conf", "/dev/null"];
        let args = args.iter().map(|s| s.to_string());

        let opts = Options::new(args).unwrap();
        assert!(! opts.is_valid());
        assert_eq!(opts.conf_path.unwrap().to_str().unwrap(), "/dev/null");
    }

    #[test]
    fn args_filter() {
        let args = vec!["upim-contact", "--filter", "Name = 'Somebody'"];
        let args = args.iter().map(|s| s.to_string());

        let opts = Options::new(args).unwrap();
        assert!(opts.is_valid());
        assert_eq!(opts.filter,
            Some(Condition::Filter(
                "Name".into(),
                FilterOp::EqualTo,
                "Somebody".into()
            ))
        );
    }

    #[test]
    fn args_chain_filters() {
        let args = vec![
            "upim-contact", "--filter", "Name = 'Somebody'",
            "--filter", "Name = 'Nobody'"
        ];
        let args = args.iter().map(|s| s.to_string());

        let opts = Options::new(args).unwrap();
        assert!(opts.is_valid());
        assert_eq!(opts.filter,
            Some(Condition::And(Box::new((
                Condition::Filter(
                    "Name".into(),
                    FilterOp::EqualTo,
                    "Somebody".into()
                ),
                Condition::Filter(
                    "Name".into(),
                    FilterOp::EqualTo,
                    "Nobody".into()
                )
            ))))
        );
    }
}
