//! all possible errors encapsulated
extern crate rusqlite;
extern crate serde_json;

/// wraps all errors that can possibly occur
#[derive(Debug)]
pub enum AppError {
    DBError(rusqlite::Error),
    JSONError(serde_json::Error),
    IOError(std::io::Error),
}

impl From<rusqlite::Error> for AppError {
    fn from(error: rusqlite::Error) -> Self {
        AppError::DBError(error)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        AppError::JSONError(error)
    }
}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        AppError::IOError(error)
    }
}
