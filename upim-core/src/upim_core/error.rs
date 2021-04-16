//! Common and core error types for uPIM libraries.

use std::{
    error::Error,
    path::PathBuf,
    fmt,
    io,
};


/// Error for file IO and parse errors.
#[derive(Debug, Clone)]
pub enum FileError {
    #[allow(clippy::upper_case_acronyms)]
    IO((PathBuf, io::ErrorKind)),
    Parse { file: PathBuf, msg: String, data: String, line: u32 },
}

impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            FileError::IO((ref file, ref e)) =>
                write!(f, "{:?} in file {}", e, file.to_string_lossy()),
            FileError::Parse { ref file, ref msg, ref data, ref line } =>
                write!(f, "{} at line {} in {}:\n\t{}"
                    , msg, line, file.to_string_lossy(), data),
        }
    }
}

impl Error for FileError {}

impl From<io::Error> for FileError {
    fn from(err: io::Error) -> FileError {
        FileError::IO((PathBuf::default(), err.kind()))
    }
}
