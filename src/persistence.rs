use crate::errors::AppError;
use rusqlite::{params, Connection, NO_PARAMS};
use std::collections::HashSet;

const SETUP_QUERY: &str = "CREATE TABLE words (word text primary key, status integer not null);\
                           CREATE INDEX word_index ON words(word);";
const INSERT_QUERY: &str = "INSERT OR IGNORE INTO words (word, status) VALUES (?1, ?2)";
const OVERWRITE_QUERY: &str = "REPLACE INTO words (word, status) VALUES (?1, ?2)";

const STATUS_ACTIVE: i64 = 0;
const STATUS_SUSPENDED_KNOWN: i64 = 1;
const STATUS_SUSPENDED_UNKNOWN: i64 = 2;
const STATUS_ADDED_EXTERNAL: i64 = 3;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum VocabStatus {
    Active,
    SuspendedKnown,
    SuspendedUnknown,
    AddedExternal,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Vocab {
    pub word: String,
    pub status: VocabStatus,
}

pub fn create_table(conn: &Connection) -> Result<(), AppError> {
    conn.execute(SETUP_QUERY, NO_PARAMS)?;
    Ok(())
}

pub fn add_external_words(conn: &Connection, words: &HashSet<&str>) -> Result<(), AppError> {
    for word in words {
        conn.execute(INSERT_QUERY, params![word, STATUS_ADDED_EXTERNAL])?;
    }
    Ok(())
}

pub fn insert_overwrite(conn: &Connection, vocab: &[Vocab]) -> Result<(), AppError> {
    for item in vocab {
        let word = &item.word;
        let status_int = status_to_int(&item.status);
        conn.execute(OVERWRITE_QUERY, params![word, status_int])?;
    }
    Ok(())
}

pub fn select_all(conn: &Connection) -> Result<HashSet<Vocab>, AppError> {
    let mut stmt = conn.prepare("SELECT * FROM words")?;
    // can not collect as hash set somehow?
    let vocab = stmt
        .query_map(NO_PARAMS, |row| {
            Ok(Vocab {
                word: row.get(0)?,
                status: int_to_status(row.get(1)?).unwrap(),
            })
        })?
        .collect::<Result<Vec<Vocab>, _>>();
    Ok(vocab?.into_iter().collect())
}

pub fn select_known(conn: &Connection) -> Result<Vec<String>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT (word) FROM words WHERE status != {}",
        STATUS_SUSPENDED_UNKNOWN
    ))?;
    // can not collect as hash set somehow?
    let known_words = stmt
        .query_map(NO_PARAMS, |row| Ok(row.get(0)?))?
        .collect::<Result<Vec<String>, _>>()?;
    Ok(known_words)
}

fn int_to_status(status: i64) -> Option<VocabStatus> {
    match status {
        STATUS_ACTIVE => Some(VocabStatus::Active),
        STATUS_ADDED_EXTERNAL => Some(VocabStatus::AddedExternal),
        STATUS_SUSPENDED_KNOWN => Some(VocabStatus::SuspendedKnown),
        STATUS_SUSPENDED_UNKNOWN => Some(VocabStatus::SuspendedUnknown),
        _ => None,
    }
}

fn status_to_int(status: &VocabStatus) -> i64 {
    match status {
        VocabStatus::Active => STATUS_ACTIVE,
        VocabStatus::SuspendedKnown => STATUS_SUSPENDED_KNOWN,
        VocabStatus::SuspendedUnknown => STATUS_SUSPENDED_UNKNOWN,
        VocabStatus::AddedExternal => STATUS_ADDED_EXTERNAL,
    }
}
