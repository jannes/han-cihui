use anyhow::Result;
use rusqlite::{params, Connection, NO_PARAMS};
use std::collections::{HashMap, HashSet};

use crate::{
    anki_access::{get_zh_notes, NoteStatus},
    zh_field_to_words, ANKIDB_PATH, NOTE_FIELD_PAIRS,
};

const SETUP_QUERY: &str = "CREATE TABLE words (word text primary key, status integer not null);\
                           CREATE INDEX word_index ON words(word);\
                           CREATE TABLE add_events (
                               id integer primary key autoincrement, 
                               date text not null, 
                               kind integer not null, 
                               added_words integer not null, 
                               added_chars integer not null
                            );";
const INSERT_WORD_QUERY: &str = "INSERT OR IGNORE INTO words (word, status) VALUES (?1, ?2)";
const OVERWRITE_WORD_QUERY: &str = "REPLACE INTO words (word, status) VALUES (?1, ?2)";
const INSERT_EVENT_QUERY: &str =
    "INSERT INTO add_events (date, kind, added_words, added_chars) VALUES (?1, ?2, ?3, ?4)";

const STATUS_ACTIVE: i64 = 0;
const STATUS_SUSPENDED_KNOWN: i64 = 1;
const STATUS_SUSPENDED_UNKNOWN: i64 = 2;
const STATUS_ADDED_EXTERNAL_KNOWN: i64 = 3;
const STATUS_ADDED_EXTERNAL_IGNORED: i64 = 4;

const KIND_SYNCED: i64 = 0;
const KIND_ADDED_KNOWN: i64 = 1;
const KIND_ADDED_IGNORED: i64 = 2;

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

    fn to_i64(&self) -> i64 {
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

pub fn create_table(conn: &Connection) -> Result<()> {
    conn.execute(SETUP_QUERY, NO_PARAMS)?;
    Ok(())
}

pub fn add_external_words(
    conn: &Connection,
    words: &HashSet<&str>,
    kind: AddedExternal,
) -> Result<()> {
    match kind {
        AddedExternal::Known => {
            for word in words {
                conn.execute(
                    INSERT_WORD_QUERY,
                    params![word, STATUS_ADDED_EXTERNAL_KNOWN],
                )?;
            }
        }
        AddedExternal::Ignored => {
            for word in words {
                conn.execute(
                    INSERT_WORD_QUERY,
                    params![word, STATUS_ADDED_EXTERNAL_IGNORED],
                )?;
            }
        }
    }
    Ok(())
}

pub fn insert_overwrite(conn: &Connection, vocab: &[Vocab]) -> Result<()> {
    for item in vocab {
        let word = &item.word;
        let status_int = item.status.to_i64();
        conn.execute(OVERWRITE_WORD_QUERY, params![word, status_int])?;
    }
    Ok(())
}

pub fn select_all(conn: &Connection) -> Result<HashSet<Vocab>> {
    let mut stmt = conn.prepare("SELECT * FROM words")?;
    // can not collect as hash set somehow?
    let vocab = stmt
        .query_map(NO_PARAMS, |row| {
            Ok(Vocab {
                word: row.get(0)?,
                status: VocabStatus::from_i64(row.get(1)?).unwrap(),
            })
        })?
        .collect::<Result<Vec<Vocab>, _>>();
    Ok(vocab?.into_iter().collect())
}

pub fn select_by_status(conn: &Connection, status: VocabStatus) -> Result<HashSet<String>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT (word) FROM words WHERE status = {}",
        status.to_i64()
    ))?;
    let known_words = stmt
        .query_map(NO_PARAMS, |row| Ok(row.get(0)?))?
        .collect::<Result<HashSet<String>, _>>()?;
    Ok(known_words)
}

pub fn select_known(conn: &Connection) -> Result<HashSet<String>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT (word) FROM words WHERE status != {}",
        STATUS_SUSPENDED_UNKNOWN
    ))?;
    let known_words = stmt
        .query_map(NO_PARAMS, |row| Ok(row.get(0)?))?
        .collect::<Result<HashSet<String>, _>>()?;
    Ok(known_words)
}

pub fn sync_anki_data(data_conn: &Connection) -> Result<()> {
    let conn = Connection::open(ANKIDB_PATH)?;
    let note_field_map: HashMap<&str, &str> = NOTE_FIELD_PAIRS.iter().cloned().collect();
    let zh_notes = get_zh_notes(&conn, &note_field_map)?;
    let anki_vocab: Vec<Vocab> = zh_notes
        .iter()
        .flat_map(|note| {
            let status = match note.status {
                NoteStatus::Active => VocabStatus::Active,
                NoteStatus::SuspendedKnown => VocabStatus::SuspendedKnown,
                NoteStatus::SuspendedUnknown => VocabStatus::SuspendedUnknown,
            };
            let words = zh_field_to_words(&note.zh_field);
            words.into_iter().map(move |word| Vocab { word, status })
        })
        .collect();
    insert_overwrite(data_conn, &anki_vocab)
}
