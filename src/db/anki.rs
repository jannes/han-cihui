extern crate rusqlite;
extern crate serde_json;

use std::{collections::HashMap, time::Instant};

use anyhow::{Context, Result};
use jieba_rs::Jieba;
use rusqlite::{params, Connection, Statement, ToSql};

use crate::{
    config::{get_config, Config, ANKI_SUSPENDED_KNOWN_FLAG, ANKI_SUSPENDED_UNKNOWN_FLAG},
    fan2jian::get_mapping,
    segmentation::extract_words,
};

use super::vocab::{db_words_insert_overwrite, VocabStatus};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum NoteStatus {
    Active,
    SuspendedKnown,
    SuspendedUnknown,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Note {
    pub fields_raw: String,
    pub status: NoteStatus,
}

pub fn db_sync_anki_data(data_conn: &Connection) -> Result<()> {
    let now = Instant::now();
    let Config {
        anki_db_path,
        anki_notes,
        ..
    } = get_config();

    let conn =
        Connection::open_with_flags(anki_db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE)?;
    let zh_notes = get_zh_notes(&conn, anki_notes)?;

    // append notes into big long text for each status type
    let mut text_active = String::new();
    let mut text_suspended_known = String::new();
    let mut text_suspended_unknown = String::new();
    for note in zh_notes {
        match note.status {
            NoteStatus::Active => text_active.push_str(&note.fields_raw),
            NoteStatus::SuspendedKnown => text_suspended_known.push_str(&note.fields_raw),
            NoteStatus::SuspendedUnknown => text_suspended_unknown.push_str(&note.fields_raw),
        }
    }

    // extract words from each big text and construct vocab
    // active > suspended known > suspended unknown
    // (e.g if word is both in active and unknown note, count as active)
    let jieba = Jieba::new();
    let fan2jian = get_mapping(true);
    let jian2fan = get_mapping(false);
    let mut vocab: HashMap<String, VocabStatus> = HashMap::new();

    for word in extract_words(&text_suspended_unknown, &jieba, &fan2jian, &jian2fan) {
        vocab.insert(word, VocabStatus::SuspendedUnknown);
    }
    for word in extract_words(&text_suspended_known, &jieba, &fan2jian, &jian2fan) {
        vocab.insert(word, VocabStatus::SuspendedKnown);
    }
    for word in extract_words(&text_active, &jieba, &fan2jian, &jian2fan) {
        vocab.insert(word, VocabStatus::Active);
    }
    let duration = now.elapsed();
    eprintln!("anki sync extraction duration: {duration:#?})");

    db_words_insert_overwrite(data_conn, &vocab)
}

/**
-------------- PRIVATE ----------------
*/

#[derive(Debug)]
struct Notetype {
    id: i64,
    name: String,
}

// cards.ord refers to card number
// cards.ord = 0 selects Card 1
// for 中文-英文 Notetype that is the Chinese->English Card
const SELECT_ACTIVE_SQL: &str = "SELECT notes.flds FROM notes JOIN cards \
            ON notes.id = cards.nid \
            WHERE notes.mid = ?1 \
            AND cards.queue != -1 \
            AND cards.ord = 0";

const SELECT_INACTIVE_SQL: &str = "SELECT notes.flds FROM notes JOIN cards \
            ON notes.id = cards.nid \
            WHERE notes.mid = ?1 \
            AND cards.queue = -1 \
            AND cards.flags = ?2 \
            AND cards.ord = 0";

const SELECT_NOTETYPES_SQL: &str = "SELECT notetypes.id, notetypes.name FROM notetypes";

fn get_zh_notes(conn: &Connection, notetypes: Vec<String>) -> Result<Vec<Note>> {
    let notetypes = get_zh_notetypes(conn, notetypes)?;
    let mut all_notes: Vec<Note> = Vec::new();
    for Notetype {
        id: notetype_id, ..
    } in notetypes
    {
        all_notes.extend(select_notes(conn, notetype_id, NoteStatus::Active)?);
        all_notes.extend(select_notes(
            conn,
            notetype_id,
            NoteStatus::SuspendedUnknown,
        )?);
        all_notes.extend(select_notes(conn, notetype_id, NoteStatus::SuspendedKnown)?);
    }
    Ok(all_notes)
}

fn get_zh_notetypes(conn: &Connection, zh_notetype_names: Vec<String>) -> Result<Vec<Notetype>> {
    let mut notetypes_query = conn.prepare(SELECT_NOTETYPES_SQL)?;
    let all_notetypes = notetypes_query.query_map(params![], |row| {
        Ok(Notetype {
            id: row.get(0)?,
            name: row.get(1)?,
        })
    })?;

    let res: Result<_, _> = all_notetypes
        .filter(|nt| {
            if nt.is_ok() {
                zh_notetype_names.contains(&nt.as_ref().unwrap().name)
            } else {
                false
            }
        })
        .collect();
    res.context("failed to select notetypes")
}

fn select_notes(
    conn: &Connection,
    note_type_id: i64,
    status: NoteStatus,
) -> Result<Vec<Note>, rusqlite::Error> {
    let stmt_to_result = |mut stmt: Statement, status: NoteStatus, params: &[&dyn ToSql]| {
        stmt.query_map(params, |row| {
            Ok(Note {
                fields_raw: row.get(0)?,
                status,
            })
        })?
        .collect::<Result<Vec<Note>, _>>()
    };
    match status {
        NoteStatus::Active => {
            let stmt = conn.prepare(SELECT_ACTIVE_SQL)?;
            let params = params![note_type_id];
            stmt_to_result(stmt, status, params)
        }
        NoteStatus::SuspendedUnknown => {
            let stmt = conn.prepare(SELECT_INACTIVE_SQL)?;
            let params = params![note_type_id, ANKI_SUSPENDED_UNKNOWN_FLAG];
            stmt_to_result(stmt, status, params)
        }
        NoteStatus::SuspendedKnown => {
            let stmt = conn.prepare(SELECT_INACTIVE_SQL)?;
            let params = params![note_type_id, ANKI_SUSPENDED_KNOWN_FLAG];
            stmt_to_result(stmt, status, params)
        }
    }
}
