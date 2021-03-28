use std::{
    path::{Path, PathBuf},
    fmt,
};

use super::config::Config;


pub fn collection_path(conf: &Config, name: &str)
-> std::result::Result<PathBuf, CollectionError> {
    if let Some(path) = conf.get("Collections", name) {
        let path = Path::new(path);

        if path.is_absolute() {
            Ok(PathBuf::from(path))
        } else if let Some(base) = conf.get_default("collection_base") {
            Ok(Path::new(base).join(path))
        } else {
            Err(CollectionError::CannotMakeAbsolutePath)
        }
    } else {
        Err(CollectionError::CollectionDoesNotExist)
    }
}

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
