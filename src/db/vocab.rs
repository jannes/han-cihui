use anyhow::Result;
use rusqlite::{params, Connection};
use std::collections::{HashMap, HashSet};

use super::anki::NoteStatus;

// vocabulary
const INSERT_WORD_QUERY: &str = "INSERT OR IGNORE INTO words (word, status, last_changed)
                                 VALUES (?1, ?2, strftime('%s','now'))";
const DELETE_WORD_QUERY: &str = "DELETE FROM words WHERE word = ?1";
const OVERWRITE_WORD_QUERY: &str = "REPLACE INTO words (word, status, last_changed)
                                    VALUES (?1, ?2, strftime('%s','now'))";

const STATUS_ACTIVE: i64 = 0;
const STATUS_SUSPENDED: i64 = 1;
const STATUS_ADDED_EXTERNAL: i64 = 2;

const SELECT_MAX_MODIFED: &str =
    "SELECT COALESCE(MAX(latest_modified), 0) as max_mod FROM anki_sync";
const INSERT_SYNC: &str = "INSERT INTO anki_sync (latest_modified) VALUES (?1)";

pub fn select_max_modified(conn: &Connection) -> Result<i64> {
    let mut stmt = conn.prepare(SELECT_MAX_MODIFED)?;
    let max_mod = stmt.query_row([], |row| row.get::<usize, i64>(0))?;
    Ok(max_mod)
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum VocabStatus {
    Active,
    Inactive,
    AddedExternal,
}

impl VocabStatus {
    fn from_i64(i: i64) -> Option<Self> {
        match i {
            STATUS_ACTIVE => Some(VocabStatus::Active),
            STATUS_SUSPENDED => Some(VocabStatus::Inactive),
            STATUS_ADDED_EXTERNAL => Some(VocabStatus::AddedExternal),
            _ => None,
        }
    }

    fn to_i64(self) -> i64 {
        match self {
            VocabStatus::Active => STATUS_ACTIVE,
            VocabStatus::Inactive => STATUS_SUSPENDED,
            VocabStatus::AddedExternal => STATUS_ADDED_EXTERNAL,
        }
    }
}

impl From<NoteStatus> for VocabStatus {
    fn from(status: NoteStatus) -> Self {
        match status {
            NoteStatus::Active => VocabStatus::Active,
            NoteStatus::Inactive => VocabStatus::Inactive,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Vocab {
    pub word: String,
    pub status: VocabStatus,
}

pub fn db_words_add_external(conn: &Connection, words: &HashSet<&str>) -> Result<()> {
    let status = STATUS_ADDED_EXTERNAL;
    for word in words {
        conn.execute(INSERT_WORD_QUERY, params![word, status])?;
    }
    Ok(())
}

pub fn db_words_delete(conn: &Connection, words: &HashSet<String>) -> Result<()> {
    for word in words {
        conn.execute(DELETE_WORD_QUERY, params![word])?;
    }
    Ok(())
}

/// Insert or update given vocabulary
/// if latest_modified is Some, record Anki sync event in same transaction
pub fn db_words_insert_overwrite(
    conn: &mut Connection,
    vocab: &HashMap<String, VocabStatus>,
    latest_modified: Option<i64>,
) -> Result<()> {
    let tx = conn.transaction()?;
    // if new words insert is result from Anki sync, record sync
    if let Some(latest_mod) = latest_modified {
        tx.execute(INSERT_SYNC, params![latest_mod])?;
    }
    for (word, status) in vocab {
        let status_int = status.to_i64();
        tx.execute(OVERWRITE_WORD_QUERY, params![word, status_int])?;
    }
    tx.commit()?;
    Ok(())
}

pub fn db_words_select_all(conn: &Connection) -> Result<HashSet<Vocab>> {
    let mut stmt = conn.prepare("SELECT * FROM words")?;
    // can not collect as hash set somehow?
    let vocab = stmt
        .query_map([], |row| {
            Ok(Vocab {
                word: row.get(0)?,
                status: VocabStatus::from_i64(row.get(1)?).unwrap(),
            })
        })?
        .collect::<Result<Vec<Vocab>, _>>();
    Ok(vocab?.into_iter().collect())
}

pub fn db_words_select_known(conn: &Connection) -> Result<HashSet<String>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT (word) FROM words WHERE status != {}",
        STATUS_SUSPENDED
    ))?;
    let known_words = stmt
        .query_map([], |row| row.get(0))?
        .collect::<Result<HashSet<String>, _>>()?;
    Ok(known_words)
}
