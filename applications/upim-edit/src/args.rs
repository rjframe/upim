//! Command-line argument parsing

use std::path::{Path, PathBuf};

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
    pub fn new<T>(args: T) -> anyhow::Result<Self>
        where T: Iterator<Item = String>,
    {
        let args = &mut args.collect::<Vec<String>>();
        let mut args = args.as_mut_slice();
        let mut opts = Self::default();

        if args.len() < 2 {
            return Err(anyhow!("Missing filename"));
        }
        args = &mut args[1..];

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
                "--tags" => {
                    opts.action = Action::PrintTags;
                    args = &mut args[1..];
                },
                "--attributes" => {
                    opts.action = Action::PrintAttributes;
                    args = &mut args[1..];
                },
                "--content" => {
                    opts.action = Action::PrintContent;
                    args = &mut args[1..];
                },
                "--add-tags" => {
                    let tags = read_tags(&args)?;
                    assert!(tags.len() < args.len());

                    args = &mut args[tags.len()+1..];
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
                    args = &mut args[3..];
                },
                "--remove-tags" => {
                    let tags = read_tags(&args)?;
                    assert!(tags.len() < args.len());

                    args = &mut args[tags.len()+1..];
                    opts.action = Action::RemoveTags(tags);
                },
                "--remove-attr" => {
                    if args.len() < 2 {
                        return Err(anyhow!("Missing attribute name"));
                    }

                    opts.action = Action::RemoveAttribute(args[1].clone());
                    args = &mut args[2..];
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
                    args = &mut args[1..];
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
        let args = vec!["upim-edit", "some-file.txt"];
        let args = args.iter().map(|s| s.to_string());

        let opts = Options::new(args).unwrap();
        assert_eq!(opts.file.to_str().unwrap(), "some-file.txt");
        assert_eq!(opts.action, Action::Edit);

        let args = vec!["upim-edit", "/tmp/some-file.txt"];
        let args = args.iter().map(|s| s.to_string());

        let opts = Options::new(args).unwrap();
        assert_eq!(opts.file.to_str().unwrap(), "/tmp/some-file.txt");
        assert_eq!(opts.action, Action::Edit);
    }

    #[test]
    fn args_specify_collection() {
        let args = vec!["upim-edit", "-C", "coll", "some-file.txt"];
        let args = args.iter().map(|s| s.to_string());

        let opts = Options::new(args).unwrap();
        assert_eq!(opts.file.to_str().unwrap(), "some-file.txt");
        assert_eq!(opts.collection.unwrap(), "coll");
        assert_eq!(opts.action, Action::Edit);
    }

    #[test]
    fn args_with_collection_path_must_be_relative() {
        let args = vec!["upim-edit", "-C", "coll", "/tmp/some-file.txt"];
        let args = args.iter().map(|s| s.to_string());

        let opts = Options::new(args);
        assert!(opts.is_err());
    }

    // TODO: The --conf path must exist; need cross-platform test.
    #[cfg(unix)]
    #[test]
    fn args_conf_path() {
        let args = vec![
            "upim-edit", "--conf", "/dev/null", "/tmp/some-file.txt"
        ];
        let args = args.iter().map(|s| s.to_string());

        let opts = Options::new(args).unwrap();
        assert_eq!(opts.conf_path.unwrap().to_str().unwrap(), "/dev/null");
        assert_eq!(opts.file.to_str().unwrap(), "/tmp/some-file.txt");
        assert_eq!(opts.action, Action::Edit);
    }

    #[test]
    fn args_tags() {
        let args = vec!["upim-edit", "--tags", "/tmp/some-file.txt"];
        let args = args.iter().map(|s| s.to_string());

        let opts = Options::new(args).unwrap();
        assert_eq!(opts.file.to_str().unwrap(), "/tmp/some-file.txt");
        assert_eq!(opts.action, Action::PrintTags);
    }

    #[test]
    fn args_attributes() {
        let args = vec!["upim-edit", "--attributes", "/tmp/some-file.txt"];
        let args = args.iter().map(|s| s.to_string());

        let opts = Options::new(args).unwrap();
        assert_eq!(opts.file.to_str().unwrap(), "/tmp/some-file.txt");
        assert_eq!(opts.action, Action::PrintAttributes);
    }

    #[test]
    fn args_content() {
        let args = vec!["upim-edit", "--content", "/tmp/some-file.txt"];
        let args = args.iter().map(|s| s.to_string());

        let opts = Options::new(args).unwrap();
        assert_eq!(opts.file.to_str().unwrap(), "/tmp/some-file.txt");
        assert_eq!(opts.action, Action::PrintContent);
    }

    #[test]
    fn args_add_tags() {
        let args = vec![
            "upim-edit", "--add-tags", "@tag1", "@tag2", "/tmp/some-file.txt"
        ];
        let args = args.iter().map(|s| s.to_string());

        let tags = vec!["@tag1".into(), "@tag2".into()];

        let opts = Options::new(args).unwrap();
        assert_eq!(opts.action, Action::AddTags(tags));
    }

    #[test]
    fn args_add_tags_missing_tags() {
        let args = vec!["upim-edit", "--add-tags", "/tmp/some-file.txt"];
        let args = args.iter().map(|s| s.to_string());

        assert!(Options::new(args).is_err());
    }

    #[test]
    fn args_add_tags_invalid_tag_name() {
        let args = vec!["upim-edit", "--add-tags", "tag1", "/tmp/some-file.txt"];
        let args = args.iter().map(|s| s.to_string());

        assert!(Options::new(args).is_err());
    }

    #[test]
    fn args_add_attribute() {
        let args = vec![
            "upim-edit", "--add-attr", "key", "value", "/tmp/some-file.txt"
        ];
        let args = args.iter().map(|s| s.to_string());

        let opts = Options::new(args).unwrap();
        assert_eq!(
            opts.action,
            Action::AddAttribute("key".into(), "value".into())
        );
    }

    #[test]
    fn args_add_attribute_missing_one_kv() {
        let args = vec!["upim-edit", "--add-attr", "key", "/tmp/some-file.txt"];
        let args = args.iter().map(|s| s.to_string());

        assert!(Options::new(args).is_err());
    }

    #[test]
    fn args_add_attribute_missing_both_kv() {
        let args = vec!["upim-edit", "--add-attr", "/tmp/some-file.txt"];
        let args = args.iter().map(|s| s.to_string());

        assert!(Options::new(args).is_err());
    }

    #[test]
    fn args_add_attribute_spaces_in_kv() {
        let args = vec![
            "upim-edit", "--add-attr", "my key", "my value",
            "/tmp/some-file.txt"
        ];
        let args = args.iter().map(|s| s.to_string());

        let opts = Options::new(args).unwrap();
        assert_eq!(
            opts.action,
            Action::AddAttribute("my key".into(), "my value".into())
        );
    }

    #[test]
    fn args_remove_tags() {
        let args = vec![
            "upim-edit", "--remove-tags", "@tag1", "@tag2", "/tmp/some-file.txt"
        ];
        let args = args.iter().map(|s| s.to_string());

        let tags = vec!["@tag1".into(), "@tag2".into()];

        let opts = Options::new(args).unwrap();
        assert_eq!(opts.action, Action::RemoveTags(tags));
    }

    #[test]
    fn args_remove_tags_missing_tags() {
        let args = vec![
            "upim-edit", "--remove-tags", "/tmp/some-file.txt"
        ];
        let args = args.iter().map(|s| s.to_string());

        assert!(Options::new(args).is_err());
    }

    #[test]
    fn args_remove_tags_invalid_tag_name() {
        let args = vec![
            "upim-edit", "--remove-tags", "tag1", "/tmp/some-file.txt"
        ];
        let args = args.iter().map(|s| s.to_string());

        assert!(Options::new(args).is_err());
    }

    #[test]
    fn args_remove_attribute() {
        let args = vec![
            "upim-edit", "--remove-attr", "key", "/tmp/some-file.txt"
        ];
        let args = args.iter().map(|s| s.to_string());

        let opts = Options::new(args).unwrap();
        assert_eq!(opts.action, Action::RemoveAttribute("key".into()));
    }

    #[test]
    fn args_remove_attribute_no_name() {
        let args = vec!["upim-edit", "--remove-attr", "/tmp/some-file.txt"];
        let args = args.iter().map(|s| s.to_string());

        assert!(Options::new(args).is_err());
    }
}
