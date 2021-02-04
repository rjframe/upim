//! Common and core error types for uPIM libraries.

use std::{
    error::Error,
    io,
    fmt,
};


/// Error for file IO and parse errors.
#[derive(Debug)]
pub enum FileError {
    IO(io::Error),
    Parse { msg: String, data: String },
}

impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            FileError::IO(ref e) => e.fmt(f),
            FileError::Parse { ref msg, ref data } =>
                write!(f, "{}:\n\t{}", msg, data),
        }
    }
}

impl Error for FileError {}

impl From<io::Error> for FileError {
    fn from(err: io::Error) -> FileError {
        FileError::IO(err)
    }
}
