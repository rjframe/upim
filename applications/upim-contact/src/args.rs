//! Command-line argument parsing

use std::{
    num::{ParseIntError, IntErrorKind},
    path::{Path, PathBuf},
    str::FromStr as _,
};

use anyhow::anyhow;

use crate::{
    either::Either,
    filter::Query,
};


// TODO: Read the configuration file separately from the rest so I can simplify
// my alias management.

/// Describes the order in which to sort a field when outputting contact
/// information.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Sort {
    NoSort,
    Ascending(String),
    Descending(String),
}

impl Default for Sort { fn default() -> Self { Self::NoSort } }

#[derive(Debug)]
pub enum Command {
    Search,
    Alias(String),
    New(String),
    Edit(Either<String, PathBuf>),
}

impl Default for Command { fn default() -> Self { Self::Search } }

#[derive(Debug, Default)]
pub struct Options {
    // A command or filter alias.
    pub cmd_or_alias: Command,
    // The parameters to substitute into the alias.
    pub alias_params: Option<Vec<String>>,
    // The non-default collection to use.
    pub collection: Option<String>,
    // An alternate configuration file.
    pub conf_path: Option<PathBuf>,
    // Multiple conditions are ANDed together and stored as one.
    // If there is no cmd_or_alias, there must be a filter.
    pub filter: Option<Query>,
    // Maximum number of records to list
    pub limit: Option<u32>,
    pub sort: Sort,
}

impl Options {
    /// Parse options from the arguments list passed from the operating system.
    ///
    /// The first element is assumed to be the application name or path and is
    /// ignored.
    pub fn new<T>(args: T) -> anyhow::Result<Self>
        where T: Iterator<Item = String>,
    {
        let mut args = args;
        args.next();
        Self::new_from_arguments(args)
    }

    /// Parse the list of arguments provided to the application.
    pub(crate) fn new_from_arguments<T>(args: T) -> anyhow::Result<Self>
        where T: Iterator<Item = String>,
    {
        let mut args = &mut args.collect::<Vec<String>>()[..];
        let mut opts = Options::default();

        while ! args.is_empty() {
            match args[0].as_ref() {
                "-C" => {
                    enforce_len(&args, 2, "Missing collection name")?;
                    opts.collection = Some(args[1].to_owned());
                    args = &mut args[2..];
                },
                "--conf" => {
                    enforce_len(&args, 2, "Missing configuration path")?;

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
                    enforce_len(&args, 2, "No query filter provided")?;

                    let filter = match opts.filter {
                        Some(f) => {
                            let new_filter = Query::from_str(&args[1])?;
                            Some(f.merge_with(new_filter))
                        },
                        None => Some(Query::from_str(&args[1])?)
                    };
                    opts.filter = filter;
                    args = &mut args[2..];
                },
                "--limit" => {
                    enforce_len(&args, 2, "Expected limit value")?;

                    let limit = match args[1].parse::<u32>() {
                        Ok(v) => if v > 0 { Some(v) } else { None },
                        Err(_) => None,
                    };
                    opts.limit = limit;
                    args = &mut args[2..];
                },
                _ => {
                    if args[0].starts_with("--sort-") && args[0].len() > 7 {
                        enforce_len(&args, 2, "Missing the field to sort by")?;

                        let sort = match args[0].chars().nth(7) {
                            Some('a') => Sort::Ascending(args[1].to_owned()),
                            Some('d') => Sort::Descending(args[1].to_owned()),
                            _ => return Err(anyhow!(
                                "Unknown sort method: {}", args[0]
                            )),
                        };

                        opts.sort = sort;
                        args = &mut args[2..];
                        continue;
                    } else if args[0] == "new" {
                        enforce_len(&args, 2,
                            "Expected a contact name for the `new` command")?;

                        opts.cmd_or_alias = Command::New(args[1].to_owned());
                        args = &mut args[2..];
                    } else if args[0] == "edit" {
                        enforce_len(&args, 2,
                            concat!("Expected a contact name or path for the ",
                                "edit command"))?;

                        // The path to edit must exist, so we can use this to
                        // validate it:
                        match Path::new(&args[1]).canonicalize() {
                            Ok(p) => {
                                opts.cmd_or_alias = Command::Edit(
                                    Either::Right(p)
                                )
                            },
                            Err(_) => {
                                // Invalid path; probably a name. Will be
                                // validated later.
                                opts.cmd_or_alias = Command::Edit(
                                    Either::Left(args[1].to_owned())
                                )
                            },
                        }
                    } else {
                        // We're going to assume a valid alias for now.
                        // We cannot verify it here because that creates a
                        // circular dependency between reading the configuration
                        // file and reading these options.
                        opts.cmd_or_alias = Command::Alias(args[0].to_owned());

                        if args.len() == 1 {
                            args = &mut args[1..];
                        } else {
                            let mut params = vec![];

                            for arg in args[1..].iter() {
                                if arg.starts_with('-') { break; }
                                params.push(arg.to_owned());
                            }

                            let (len, params) = if params.is_empty() {
                                (0, None)
                            } else {
                                (params.len(), Some(params))
                            };
                            opts.alias_params = params;

                            args = &mut args[len+1..];
                        }
                    }
                }
            }
        }

        Ok(opts)
    }
}

/// Errors that can be returned when attempting to perform parameter
/// substitution in aliases.
#[derive(Debug)]
pub enum AliasSubstitutionError {
    /// An error attempting to parse an error as an integer.
    ParseError(IntErrorKind),
    /// A command-line argument was not provided as a value to the specified
    /// parameter substitution index.
    MissingValue(usize),
}

impl std::fmt::Display for AliasSubstitutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {

        match self {
            Self::ParseError(kind) =>
                write!(f, "Error parsing parameter index: {:?}", kind),
            Self::MissingValue(i) =>
                write!(f, "No value provided for the parameter ${}", i),
        }
    }
}

impl std::error::Error for AliasSubstitutionError {}

impl From<ParseIntError> for AliasSubstitutionError {
    fn from(e: ParseIntError) -> Self {
        Self::ParseError(e.kind().to_owned())
    }
}

/// Substitute strings from command-line arguments into the given alias.
///
/// The name of the alias itself should not be passed in `args`.
///
/// # Returns
///
/// Returns a tuple of the new alias string and the number of arguments read.
pub fn substitute_alias(args: &[String], alias: &str)
-> std::result::Result<(usize, String), AliasSubstitutionError> {
    let mut new_alias = String::default();
    let mut idx = 0;
    let mut last = 0;

    for (i, _) in alias.match_indices('$') {
        // TODO: Check optimizer on this. The implementation iterates the
        // string; I'd be better off to do `if i > 0 && alias[i..i+1]`.
        if alias.chars().nth(i-1) == Some('\\') { continue; }

        let num = alias[i+1..].chars()
            .take_while(|c| c.is_numeric())
            .collect::<String>();

        let num_len = num.len() + 1; // Count the dollar sign.
        let num = num.parse::<usize>()?;

        if num >= args.len() {
            return Err(AliasSubstitutionError::MissingValue(num));
        }

        new_alias.push_str(&alias[last..i]);
        new_alias.push_str(&args[num]);

        idx += 1;
        last = i + num_len;
    };

    new_alias.push_str(&alias[last..]);

    Ok((idx, new_alias))
}

impl Options {
    /// Determine whether this is a valid [Options] object.
    ///
    /// If a command or alias was not provided ([Command::Search]), then there
    /// must be a filter. If there is a command/alias, a filter is optional.
    #[allow(clippy::len_zero)]
    fn is_valid(&self) -> bool {
        ! matches!(self.cmd_or_alias, Command::Search)
            || self.filter.is_some()
    }
}

#[inline]
fn enforce_len<T>(arr: &[T], cnt: usize, msg: &str) -> anyhow::Result<()> {
    if arr.len() < cnt {
        Err(anyhow!("{}", msg))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::filter::{Condition, FilterOp};

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
    fn args_filter_no_where_clause() {
        let args = vec!["upim-contact", "--filter", "Name,Phone"];
        let args = args.iter().map(|s| s.to_string());

        let opts = Options::new(args).unwrap();
        assert!(opts.is_valid());
        assert_eq!(opts.filter,
            Some(Query {
                select: vec!["Name".into(), "Phone".into()],
                condition: Condition::All,
            })
        );
    }

    #[test]
    fn args_filter() {
        let args = vec!["upim-contact", "--filter",
            "'Name,Phone' WHERE Name = 'Somebody'"
        ];
        let args = args.iter().map(|s| s.to_string());

        let opts = Options::new(args).unwrap();
        assert!(opts.is_valid());
        assert_eq!(opts.filter,
            Some(Query {
                select: vec!["Name".into(), "Phone".into()],
                condition: Condition::Filter(
                    "Name".into(),
                    FilterOp::EqualTo,
                    "Somebody".into()
                )
            })
        );
    }

    #[test]
    fn args_filter_all_fields() {
        let args = vec!["upim-contact", "--filter",
            "* WHERE Name = 'Somebody'"
        ];
        let args = args.iter().map(|s| s.to_string());

        let opts = Options::new(args).unwrap();
        assert!(opts.is_valid());
        assert_eq!(opts.filter,
            Some(Query {
                select: vec!["*".into()],
                condition: Condition::Filter(
                    "Name".into(),
                    FilterOp::EqualTo,
                    "Somebody".into()
                )
            })
        );
    }

    #[test]
    fn args_chain_filters() {
        let args = vec![
            "upim-contact", "--filter",
            "Name,Phone WHERE Name = 'Somebody'",
            "--filter", "Name,Address WHERE Name = 'Nobody'"
        ];
        let args = args.iter().map(|s| s.to_string());

        let opts = Options::new(args).unwrap();
        assert!(opts.is_valid());
        assert_eq!(opts.filter,
            Some(Query {
                select: vec!["Name".into()],
                condition:
                    Condition::And(Box::new((
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
                ))),
            })
        );
    }

    #[test]
    fn args_limit() {
        let args = vec!["upim-contact", "--limit", "2"];
        let args = args.iter().map(|s| s.to_string());

        let opts = Options::new(args).unwrap();
        assert_eq!(opts.limit, Some(2));
    }

    #[test]
    fn args_limit_zero_is_ignored() {
        let args = vec!["upim-contact", "--limit", "0"];
        let args = args.iter().map(|s| s.to_string());

        let opts = Options::new(args).unwrap();
        assert_eq!(opts.limit, None);
    }

    #[test]
    fn args_invalid_limit_is_ignored() {
        let args = vec!["upim-contact", "--limit", "-4"];
        let args = args.iter().map(|s| s.to_string());

        let opts = Options::new(args).unwrap();
        assert_eq!(opts.limit, None);

        let args = vec!["upim-contact", "--limit", "asdf"];
        let args = args.iter().map(|s| s.to_string());

        let opts = Options::new(args).unwrap();
        assert_eq!(opts.limit, None);
    }

    #[test]
    fn args_sort_ascending() {
        let args = vec!["upim-contact", "--sort-a", "Name"];
        let args = args.iter().map(|s| s.to_string());

        let opts = Options::new(args).unwrap();
        assert_eq!(opts.sort, Sort::Ascending("Name".into()));
    }

    #[test]
    fn args_sort_descending() {
        let args = vec!["upim-contact", "--sort-d", "Name"];
        let args = args.iter().map(|s| s.to_string());

        let opts = Options::new(args).unwrap();
        assert_eq!(opts.sort, Sort::Descending("Name".into()));
    }

    #[test]
    fn args_invalid_sort_is_err() {
        let args = vec!["upim-contact", "--sort-b", "Name"];
        let args = args.iter().map(|s| s.to_string());

        assert!(Options::new(args).is_err());
    }

    #[test]
    fn substitute_alias_no_substitution() {
        let args = vec!["--limit", "1"];
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();

        let alias = "--filter 'Name,Phone' WHERE Name = 'Nobody'";

        let (len, alias) = substitute_alias(&args, alias).unwrap();
        assert_eq!(len, 0);
        assert_eq!(alias, "--filter 'Name,Phone' WHERE Name = 'Nobody'");
    }

    #[test]
    fn substitute_alias_values() {
        let args = vec!["Some Person"];
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();

        let alias = "--filter 'Name,Phone' WHERE Name = '$0' --limit 1";

        let (len, new_alias) = substitute_alias(&args, alias).unwrap();
        assert_eq!(len, 1);
        assert_eq!(new_alias,
            "--filter 'Name,Phone' WHERE Name = 'Some Person' --limit 1"
        );
    }

    #[test]
    fn substitute_alias_escaped_dollar() {
        let args = vec!["Some Person"];
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();

        let alias = "--filter 'Name,Phone' WHERE Name = '\\$0' --limit 1";

        let (len, new_alias) = substitute_alias(&args, alias).unwrap();
        assert_eq!(len, 0);
        assert_eq!(new_alias,
            "--filter 'Name,Phone' WHERE Name = '\\$0' --limit 1"
        );
    }

    #[test]
    fn subsitute_multiple_aliases() {
        let args = vec!["Some Person", "12345"];
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();

        let alias = "--filter 'Name,Phone' WHERE Name = '$0' AND Phone = '$1'";

        let (len, new_alias) = substitute_alias(&args, alias).unwrap();
        assert_eq!(len, 2);
        assert_eq!(new_alias,
            "--filter 'Name,Phone' WHERE Name = 'Some Person' \
            AND Phone = '12345'"
        );
    }

    #[test]
    fn subsitute_alias_missing_value_is_err() {
        let args = vec!["Some Person"];
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();

        let alias = "--filter 'Name,Phone' WHERE Name = '$0' AND Phone = '$1'";

        assert!(substitute_alias(&args, alias).is_err());
    }

    #[test]
    fn substitute_alias_extra_values_are_ignored() {
        let args = vec!["Some Person", "12345"];
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();

        let alias = "--filter 'Name,Phone' WHERE Name = '$0' --limit 1";

        let (len, new_alias) = substitute_alias(&args, alias).unwrap();
        assert_eq!(len, 1);
        assert_eq!(new_alias,
            "--filter 'Name,Phone' WHERE Name = 'Some Person' --limit 1"
        );
    }

    #[test]
    fn substitute_alias_missing_index_is_err() {
        let args = vec!["Some Person"];
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();

        let alias = "--filter 'Name,Phone' WHERE Name = '$' --limit 1";
        assert!(substitute_alias(&args, alias).is_err());
    }

    #[test]
    fn substitute_alias_invalid_index_is_err() {
        let args = vec!["Some Person"];
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();

        let alias = "--filter 'Name,Phone' WHERE Name = '$a' --limit 1";
        assert!(substitute_alias(&args, alias).is_err());
    }
}
