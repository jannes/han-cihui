use anyhow::Result;
use rusqlite::{params, Connection};
use std::collections::{HashMap, HashSet};

use super::anki::NoteStatus;

// vocabulary
const DELETE_ANKI_WORDS_QUERY: &str = "DELETE FROM words_anki";
const INSERT_ANKI_WORD_QUERY: &str = "INSERT INTO words_anki (word, status)
                                    VALUES (?1, ?2)";

const INSERT_EXT_WORD_QUERY: &str = "INSERT OR IGNORE INTO words_external (word, status)
                                 VALUES (?1, ?2))";
const DELETE_EXT_WORD_QUERY: &str = "DELETE FROM words_external WHERE word = ?1";

const STATUS_ACTIVE: i64 = 0;
const STATUS_INACTIVE: i64 = 1;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum VocabStatus {
    Active,
    Inactive,
    AddedExternal,
}

impl VocabStatus {
    fn from_i64(i: i64) -> Self {
        match i {
            STATUS_ACTIVE => VocabStatus::Active,
            STATUS_INACTIVE => VocabStatus::Inactive,
            _ => unreachable!(),
        }
    }

    fn to_i64(self) -> i64 {
        match self {
            VocabStatus::Active => STATUS_ACTIVE,
            VocabStatus::Inactive => STATUS_INACTIVE,
            VocabStatus::AddedExternal => unreachable!(),
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
pub struct AnkiWord {
    pub word: String,
    pub status: NoteStatus,
}

pub fn db_words_external_add(conn: &Connection, words: &HashSet<&str>) -> Result<()> {
    for word in words {
        conn.execute(INSERT_EXT_WORD_QUERY, params![word])?;
    }
    Ok(())
}

pub fn db_words_external_del(conn: &Connection, words: &HashSet<String>) -> Result<()> {
    for word in words {
        conn.execute(DELETE_EXT_WORD_QUERY, params![word])?;
    }
    Ok(())
}

/// Delete all previous Anki words and insert given set
pub fn db_words_anki_update(
    conn: &mut Connection,
    vocab: &HashMap<String, VocabStatus>,
) -> Result<()> {
    let tx = conn.transaction()?;
    tx.execute(DELETE_ANKI_WORDS_QUERY, params![])?;
    for (word, status) in vocab {
        let status_int = status.to_i64();
        tx.execute(INSERT_ANKI_WORD_QUERY, params![word, status_int])?;
    }
    tx.commit()?;
    Ok(())
}

pub fn db_words_select_all(conn: &Connection) -> Result<HashMap<String, VocabStatus>> {
    let mut stmt = conn.prepare("SELECT * FROM words_anki")?;
    let mut words: HashMap<String, VocabStatus> = stmt
        .query_map([], |row| {
            let word: String = row.get(0)?;
            let status: i64 = row.get(1)?;
            Ok((word, VocabStatus::from_i64(status)))
        })?
        .collect::<std::result::Result<HashMap<String, VocabStatus>, _>>()?;

    let mut stmt = conn.prepare("SELECT * FROM words_external")?;
    for word_external in stmt.query_map([], |row| {
        let word: String = row.get(0)?;
        Ok(word)
    })? {
        words
            .entry(word_external?)
            .and_modify(|status| {
                // added external overrides inactive, but not active
                // i.e word that is both inactive and added external counts as known
                if matches!(status, VocabStatus::Inactive) {
                    *status = VocabStatus::AddedExternal;
                }
            })
            .or_insert(VocabStatus::AddedExternal);
    }

    Ok(words)
}

pub fn db_words_select_known(conn: &Connection) -> Result<HashSet<String>> {
    let vocab = db_words_select_all(conn)?;
    Ok(vocab
        .into_iter()
        .filter_map(|(word, status)| {
            if !matches!(status, VocabStatus::Inactive) {
                Some(word)
            } else {
                None
            }
        })
        .collect())
}
