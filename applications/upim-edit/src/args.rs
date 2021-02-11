//! Command-line argument parsing

use std::{
    path::{Path, PathBuf},
    env,
};

use anyhow::anyhow;


#[derive(Debug, Eq, PartialEq)]
pub enum Action {
    Edit,
    AddTags(Vec<String>),
    AddAttribute(String, String),
    RemoveTags(Vec<String>),
    RemoveAttribute(String),
    PrintTags,
    PrintAttributes,
    PrintContent,
    PrintHelp,
}

impl Default for Action {
    fn default() -> Action { Action::Edit }
}

#[derive(Debug, Default)]
pub struct Options {
    pub file: PathBuf,
    pub collection: Option<String>,
    pub conf_path: Option<PathBuf>,
    pub action: Action,
}

impl Options {
    pub fn from_args(args: env::Args) -> anyhow::Result<Self> {
        let args: Vec<String> = args.collect();
        Self::new(&args)
    }

    pub fn new(args: &[String]) -> anyhow::Result<Self> {
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
                "--content" => {
                    opts.action = Action::PrintContent;
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
                "--help" => {
                    opts.action = Action::PrintHelp;
                    break;
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

    pub fn is_valid(&self) -> bool {
        self.action == Action::PrintHelp || self.file != PathBuf::default() &&
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
    fn args_content() {
        let args = vec!["upim-edit".into(),
            "--content".into(), "/tmp/some-file.txt".into()
        ];

        let opts = Options::new(&args).unwrap();
        assert_eq!(opts.file.to_str().unwrap(), "/tmp/some-file.txt");
        assert_eq!(opts.action, Action::PrintContent);
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
