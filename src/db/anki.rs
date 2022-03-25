extern crate rusqlite;
extern crate serde_json;

use anyhow::Result;
use rusqlite::{params, Connection, Statement, ToSql};
use std::{
    collections::{HashMap, HashSet},
    convert::TryInto,
};

use crate::{
    config::{
        ANKIDB_PATH, ANKI_NOTE_FIELD_PAIRS, ANKI_SUSPENDED_KNOWN_FLAG, ANKI_SUSPENDED_UNKNOWN_FLAG,
        ANKI_WORD_DELIMITERS,
    },
    extraction::contains_hanzi,
};

use super::vocab::{db_words_insert_overwrite, Vocab, VocabStatus};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum NoteStatus {
    Active,
    SuspendedKnown,
    SuspendedUnknown,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ZhNote {
    pub zh_field: String,
    pub status: NoteStatus,
}

pub fn db_sync_anki_data(data_conn: &Connection) -> Result<()> {
    let conn = Connection::open(ANKIDB_PATH)?;
    let note_field_map: HashMap<&str, &str> = ANKI_NOTE_FIELD_PAIRS.iter().cloned().collect();
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
    db_words_insert_overwrite(data_conn, &anki_vocab)
}

/**
-------------- PRIVATE ----------------
*/

#[derive(Debug)]
struct NotetypeField {
    notetype_id: i64,
    notetype_name: String,
    field_name: String,
    field_order: usize,
}

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

const SELECT_NOTETYPES_SQL: &str = "SELECT notetypes.id, notetypes.name, FIELDS.name, FIELDS.ord \
            FROM notetypes JOIN FIELDS \
            ON notetypes.id = FIELDS.ntid";

fn zh_field_to_words(field: &str) -> Vec<String> {
    field
        .split(|c: char| !contains_hanzi(&c.to_string()) && c != '，')
        .map(String::from)
        .collect()
}

fn get_zh_notes(
    conn: &Connection,
    note_field_map: &HashMap<&str, &str>,
) -> Result<HashSet<ZhNote>> {
    let fields_info = get_zh_fields(conn, note_field_map)?;
    let mut all_notes: HashSet<ZhNote> = HashSet::new();
    for notetype_field in fields_info {
        let notetype_id = notetype_field.notetype_id;
        let field_index = notetype_field.field_order;
        all_notes.extend(select_notes(
            conn,
            notetype_id,
            field_index,
            NoteStatus::Active,
        )?);
        all_notes.extend(select_notes(
            conn,
            notetype_id,
            field_index,
            NoteStatus::SuspendedUnknown,
        )?);
        all_notes.extend(select_notes(
            conn,
            notetype_id,
            field_index,
            NoteStatus::SuspendedKnown,
        )?);
    }
    Ok(all_notes)
}

fn select_notes(
    conn: &Connection,
    note_type_id: i64,
    field_index: usize,
    status: NoteStatus,
) -> Result<Vec<ZhNote>, rusqlite::Error> {
    let stmt_to_result = |mut stmt: Statement, status: NoteStatus, params: &[&dyn ToSql]| {
        stmt.query_map(params, |row| {
            let fields_str: String = row.get(0)?;
            let zh_field: String = fieldsstr_to_field(fields_str.as_str(), field_index);
            Ok(ZhNote { zh_field, status })
        })?
        .collect::<Result<Vec<ZhNote>, _>>()
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

/// get the field at given index from a Anki fields string
/// Anki fields strings use 0x1f character as separator
fn fieldsstr_to_field(fields: &str, index: usize) -> String {
    let sep = 31u8 as char;
    let split = fields.split(sep).collect::<Vec<_>>();
    String::from(*split.get(index).unwrap())
}

fn get_zh_fields(
    conn: &Connection,
    note_field_map: &HashMap<&str, &str>,
) -> Result<Vec<NotetypeField>> {
    let mut notetypes_query = conn.prepare(SELECT_NOTETYPES_SQL)?;
    let notetype_fields = notetypes_query.query_map(params![], |row| {
        let field_order: i64 = row.get(3)?;
        Ok(NotetypeField {
            notetype_id: row.get(0)?,
            notetype_name: row.get(1)?,
            field_name: row.get(2)?,
            field_order: field_order.try_into().unwrap(),
        })
    })?;
    Ok(notetype_fields
        .map(|nf| nf.unwrap())
        .filter(|nf| {
            note_field_map.contains_key(nf.notetype_name.as_str())
                && note_field_map.get(nf.notetype_name.as_str()).unwrap() == &nf.field_name
        })
        .collect())
}
