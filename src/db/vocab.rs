use anyhow::Result;
use rusqlite::{params, Connection};
use std::collections::{HashMap, HashSet};

// vocabulary
const INSERT_WORD_QUERY: &str = "INSERT OR IGNORE INTO words (word, status, last_changed)
                                 VALUES (?1, ?2, strftime('%s','now'))";
const DELETE_WORD_QUERY: &str = "DELETE FROM words WHERE word = ?1";
const OVERWRITE_WORD_QUERY: &str = "REPLACE INTO words (word, status, last_changed)
                                    VALUES (?1, ?2, strftime('%s','now'))";

const STATUS_ACTIVE: i64 = 0;
const STATUS_SUSPENDED_KNOWN: i64 = 1;
const STATUS_SUSPENDED_UNKNOWN: i64 = 2;
const STATUS_ADDED_EXTERNAL_KNOWN: i64 = 3;
const STATUS_ADDED_EXTERNAL_IGNORED: i64 = 4;

// const SETUP_EVENT_QUERY: &str = "
//                            CREATE TABLE add_events (
//                                id integer primary key autoincrement,
//                                date text not null,
//                                kind integer not null,
//                                added_words integer not null,
//                                added_chars integer not null
//                             );";
// const INSERT_EVENT_QUERY: &str =
//     "INSERT INTO add_events (date, kind, added_words, added_chars) VALUES (?1, ?2, ?3, ?4)";

// const KIND_SYNCED: i64 = 0;
// const KIND_ADDED_KNOWN: i64 = 1;
// const KIND_ADDED_IGNORED: i64 = 2;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum VocabStatus {
    Active,
    SuspendedKnown,
    SuspendedUnknown,
    AddedExternal(AddedExternal),
}

impl VocabStatus {
    fn from_i64(i: i64) -> Option<Self> {
        match i {
            STATUS_ACTIVE => Some(VocabStatus::Active),
            STATUS_SUSPENDED_KNOWN => Some(VocabStatus::SuspendedKnown),
            STATUS_SUSPENDED_UNKNOWN => Some(VocabStatus::SuspendedUnknown),
            STATUS_ADDED_EXTERNAL_KNOWN => Some(VocabStatus::AddedExternal(AddedExternal::Known)),
            STATUS_ADDED_EXTERNAL_IGNORED => {
                Some(VocabStatus::AddedExternal(AddedExternal::Ignored))
            }
            _ => None,
        }
    }

    fn to_i64(self) -> i64 {
        match self {
            VocabStatus::Active => STATUS_ACTIVE,
            VocabStatus::SuspendedKnown => STATUS_SUSPENDED_KNOWN,
            VocabStatus::SuspendedUnknown => STATUS_SUSPENDED_UNKNOWN,
            VocabStatus::AddedExternal(AddedExternal::Known) => STATUS_ADDED_EXTERNAL_KNOWN,
            VocabStatus::AddedExternal(AddedExternal::Ignored) => STATUS_ADDED_EXTERNAL_IGNORED,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum AddedExternal {
    Known,
    Ignored,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Vocab {
    pub word: String,
    pub status: VocabStatus,
}

pub fn db_words_add_external(
    conn: &Connection,
    words: &HashSet<&str>,
    kind: AddedExternal,
) -> Result<()> {
    let status = match kind {
        AddedExternal::Known => STATUS_ADDED_EXTERNAL_KNOWN,
        AddedExternal::Ignored => STATUS_ADDED_EXTERNAL_IGNORED,
    };
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

pub fn db_words_insert_overwrite(
    conn: &Connection,
    vocab: &HashMap<String, VocabStatus>,
) -> Result<()> {
    for (word, status) in vocab {
        let status_int = status.to_i64();
        conn.execute(OVERWRITE_WORD_QUERY, params![word, status_int])?;
    }
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
        STATUS_SUSPENDED_UNKNOWN
    ))?;
    let known_words = stmt
        .query_map([], |row| row.get(0))?
        .collect::<Result<HashSet<String>, _>>()?;
    Ok(known_words)
}
