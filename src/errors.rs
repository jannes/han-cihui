//! all possible errors encapsulated
extern crate rusqlite;
extern crate serde_json;

/// wraps all errors that can possibly occur
#[derive(Debug)]
pub enum AppError {
    DBError(rusqlite::Error),
    JSONError(serde_json::Error),
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

// impl From<std::option::NoneError> for AppError {
//     fn from(error: std::option::NoneError) -> Self {
//         AppError::NoneError(error)
//     }
// }
