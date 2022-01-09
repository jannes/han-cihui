use anyhow::Result;
use rusqlite::{params, Connection};
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, SystemTime},
};

use crate::{
    analysis::AnalysisQuery,
    anki_access::{get_zh_notes, NoteStatus},
    cli_commands::zh_field_to_words,
    word_lists::{ChapterWords, WordList, WordListMetadata},
    ANKIDB_PATH, NOTE_FIELD_PAIRS,
};

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

// word lists
const INSERT_WORD_LIST_QUERY: &str = "
INSERT INTO word_lists
(book_name, author_name, create_time, min_occurrence_words, min_occurrence_chars, word_list_json)
VALUES (?1, ?2, strftime('%s', 'now'), ?3, ?4, ?5)";

const SELECT_ALL_WORD_LISTS_QUERY: &str = "
SELECT id, book_name, author_name, create_time, min_occurrence_words, min_occurrence_chars
FROM word_lists";

const SELECT_WORD_LIST_QUERY: &str = "
SELECT (word_list_json)
FROM word_lists WHERE id = ?1";

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

pub fn add_external_words(
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

pub fn delete_words(conn: &Connection, words: &HashSet<String>) -> Result<()> {
    for word in words {
        conn.execute(DELETE_WORD_QUERY, params![word])?;
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
        .query_map([], |row| {
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
        .query_map([], |row| row.get(0))?
        .collect::<Result<HashSet<String>, _>>()?;
    Ok(known_words)
}

pub fn select_known(conn: &Connection) -> Result<HashSet<String>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT (word) FROM words WHERE status != {}",
        STATUS_SUSPENDED_UNKNOWN
    ))?;
    let known_words = stmt
        .query_map([], |row| row.get(0))?
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

pub fn insert_word_list(conn: &Connection, word_list: WordList) -> Result<()> {
    let book_name = word_list.metadata.book_name;
    let author_name = word_list.metadata.author_name;
    let min_occ_words = word_list.metadata.analysis_query.min_occurrence_words;
    let min_occ_chars = word_list
        .metadata
        .analysis_query
        .min_occurrence_unknown_chars;
    let word_list_json = serde_json::to_string(&word_list.words_per_chapter)
        .expect("failed to serialize words per chapter lists");
    conn.execute(
        INSERT_WORD_LIST_QUERY,
        params![
            book_name,
            author_name,
            min_occ_words,
            min_occ_chars,
            word_list_json
        ],
    )?;
    Ok(())
}

pub fn select_all_word_lists_metadata(conn: &Connection) -> Result<Vec<WordListMetadata>> {
    let mut query = conn.prepare(SELECT_ALL_WORD_LISTS_QUERY)?;
    let res = query
        .query_map([], |row| {
            let create_time = SystemTime::UNIX_EPOCH
                .checked_add(Duration::from_secs(row.get(3)?))
                .expect("system time should not be out of bounds");
            let min_occurrence_words = row.get(4)?;
            let min_occurrence_unknown_chars = row.get(5)?;
            let analysis_query = AnalysisQuery {
                min_occurrence_words,
                min_occurrence_unknown_chars,
            };
            Ok(WordListMetadata {
                id: row.get(0)?,
                book_name: row.get(1)?,
                author_name: row.get(2)?,
                create_time,
                analysis_query,
            })
        })?
        .collect::<Result<Vec<WordListMetadata>, _>>()?;
    Ok(res)
}

pub fn select_word_list_by_id(
    conn: &Connection,
    word_list_id: u64,
) -> Result<Option<Vec<ChapterWords>>> {
    let mut query = conn.prepare(SELECT_WORD_LIST_QUERY)?;
    let res = query
        .query_map([word_list_id], |row| {
            let words_per_chapter_json: String = row.get(0)?;
            let words_per_chapter: Vec<ChapterWords> =
                serde_json::from_str(&words_per_chapter_json)
                    .expect("failed to deserialize words per chapter lists");
            Ok(words_per_chapter)
        })?
        .next()
        .transpose()?;
    Ok(res)
}
