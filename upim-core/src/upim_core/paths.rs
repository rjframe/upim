use std::{
    path::{Path, PathBuf},
    fmt,
};

use super::config::Config;


pub fn collection_path(conf: &Config, name: &str)
-> std::result::Result<PathBuf, CollectionError> {
    if let Some(path) = conf.get("Collections", name) {
        let path = expand_tilde(Path::new(path))
            .ok_or(CollectionError::CannotMakeAbsolutePath)?;

        if path.is_absolute() {
            Ok(path)
        } else if let Some(base) = conf.get_default("collection_base") {
            Ok(Path::new(base).join(path))
        } else {
            Err(CollectionError::CannotMakeAbsolutePath)
        }
    } else {
        Err(CollectionError::CollectionDoesNotExist)
    }
}

pub fn home_dir() -> Option<PathBuf> {
    // TODO: Should I convert from an Option to a Result? Thus far I've treated
    // None as an error when using this function.
    home::home_dir()
}

/// Expand the tilde in a path to the user's home directory.
pub fn expand_tilde(path: &Path) -> Option<PathBuf> {
    if path.starts_with("~") {
        home_dir()
            .map(|h| h.join(path.strip_prefix("~").unwrap()))
    } else {
        Some(path.into())
    }
}

// TODO: Move to error.rs
#[derive(Copy, Clone, Debug)]
pub enum CollectionError {
    /// Raised when a relative path is given and `collection_base` is not set in
    /// the configuration.
    CannotMakeAbsolutePath,
    /// The provided collection name is not present in the configuration.
    CollectionDoesNotExist,
}

impl fmt::Display for CollectionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CollectionError::CannotMakeAbsolutePath =>
                write!(f, "Relative collection path given without \
                    `collection_base` set in configuration"),
            CollectionError::CollectionDoesNotExist =>
                write!(f, "Collection is not present in configuration"),
        }
    }
}

impl std::error::Error for CollectionError {}

mod tests {
    use super::*;

    #[test]
    fn test_expand_tilde() {
        assert_eq!(
            expand_tilde(Path::new("~/my/path")),
            home_dir().map(|p| p.join(Path::new("my/path")))
        );
    }

    #[test]
    fn expand_tilde_ignored_in_path() {
        assert_eq!(
            expand_tilde(Path::new("my/~/path")).unwrap(),
            Path::new("my/~/path")
        );
    }
}
