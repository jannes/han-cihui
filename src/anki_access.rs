extern crate rusqlite;
extern crate serde_json;

pub use crate::errors::AppError;
use rusqlite::{params, Connection, Statement, ToSql, NO_PARAMS};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct Deck {
    pub id: i64,
    pub name: String,
}

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

#[allow(dead_code)]
pub fn get_decks(conn: &Connection) -> Result<Vec<Deck>, AppError> {
    let mut example_query = conn.prepare("SELECT decks FROM col")?;
    let mut cols = example_query.query_map(NO_PARAMS, |row| {
        Ok(DecksWrapper {
            json_str: row.get(0)?,
        })
    })?;
    let only_row = cols.next().expect("should have one row");
    jsondecks_to_decks(only_row?.json_str).map_err(AppError::from)
}

pub fn get_zh_notes(
    conn: &Connection,
    note_field_map: &HashMap<&str, &str>,
) -> Result<HashSet<ZhNote>, AppError> {
    let fields_info = get_zh_fields_info(conn, note_field_map)?;
    let mut all_notes: HashSet<ZhNote> = HashSet::new();
    for i in 0..fields_info.amount {
        let note_type_id = *fields_info.note_ids.get(i).unwrap();
        let field_index = *fields_info.zh_field_indexes.get(i).unwrap() as usize;
        all_notes.extend(select_notes(
            conn,
            note_type_id,
            field_index,
            NoteStatus::Active,
        )?);
        all_notes.extend(select_notes(
            conn,
            note_type_id,
            field_index,
            NoteStatus::SuspendedUnknown,
        )?);
        all_notes.extend(select_notes(
            conn,
            note_type_id,
            field_index,
            NoteStatus::SuspendedKnown,
        )?);
    }
    Ok(all_notes)
}

/**
-------------- PRIVATE ----------------
*/

#[derive(Debug)]
struct DecksWrapper {
    json_str: String,
}

#[derive(Debug)]
struct ZhFieldsInfo {
    note_ids: Vec<i64>,
    zh_field_indexes: Vec<i64>,
    amount: usize,
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
            let params = params![note_type_id, crate::SUSPENDED_UNKNOWN_FLAG];
            stmt_to_result(stmt, status, params)
        }
        NoteStatus::SuspendedKnown => {
            let stmt = conn.prepare(SELECT_INACTIVE_SQL)?;
            let params = params![note_type_id, crate::SUSPENDED_KNOWN_FLAG];
            stmt_to_result(stmt, status, params)
        }
    }
}

#[allow(dead_code)]
/// given a vector of i64 e.g [i1, i2, i3] return single string "<i1>, <i2>, <i3>"
fn id_vec_to_sql_set(ids: &Vec<i64>) -> String {
    match ids.get(0) {
        Some(i) => ids[1..]
            .into_iter()
            .fold(i.to_string(), |acc, &i2| acc + ", " + &i2.to_string()),
        None => String::new(),
    }
}

/// get the field at given index from a Anki fields string
/// Anki fields strings use 0x1f character as separator
fn fieldsstr_to_field(fields: &str, index: usize) -> String {
    let sep = 31u8 as char;
    let split = fields.split(sep).collect::<Vec<_>>();
    String::from(*split.get(index).unwrap())
}

fn get_zh_fields_info(
    conn: &Connection,
    note_field_map: &HashMap<&str, &str>,
) -> Result<ZhFieldsInfo, AppError> {
    let mut notes_query = conn.prepare("SELECT models FROM col")?;
    let mut rows = notes_query.query(NO_PARAMS)?;
    let notes_json_str: String = rows.next()?.unwrap().get_unwrap(0);
    let parsed_notes: Value = serde_json::from_str(notes_json_str.as_str()).unwrap();
    let notes_map = parsed_notes
        .as_object()
        .expect("this should be a json object");

    let mut note_ids = Vec::new();
    let mut zh_field_indexes = Vec::new();
    let mut amount = 0;

    // search for each target note and its associated field
    for note_id in notes_map.keys() {
        let note = notes_map.get(note_id).unwrap().as_object().unwrap();
        let note_name = note.get("name").unwrap().as_str().unwrap();
        if note_field_map.contains_key(note_name) {
            let target_field = note_field_map.get(note_name).unwrap();
            let fields = note.get("flds").unwrap().as_array().unwrap();
            for field in fields {
                let field_object = field.as_object().unwrap();
                let field_name = field_object.get("name").unwrap().as_str().unwrap();
                if *target_field == field_name {
                    let note_id_int: i64 = note.get("id").unwrap().as_i64().unwrap();
                    let field_index = field_object.get("ord").unwrap().as_i64().unwrap();
                    note_ids.push(note_id_int);
                    zh_field_indexes.push(field_index);
                    amount += 1;
                }
            }
        }
    }
    Ok(ZhFieldsInfo {
        note_ids,
        zh_field_indexes,
        amount,
    })
}

#[allow(dead_code)]
fn jsondecks_to_decks(json_str: String) -> Result<Vec<Deck>, serde_json::Error> {
    let parsed_json: Value = serde_json::from_str(json_str.as_str())?;
    let arr = parsed_json.as_object();
    match arr {
        Some(elements) => elements.values().map(jsondeck_to_deck).collect(),
        None => panic!("expected a valid json array of decks"),
    }
}

#[allow(dead_code)]
fn jsondeck_to_deck(json_deck: &Value) -> Result<Deck, serde_json::Error> {
    Ok(Deck {
        id: json_deck.get("id").unwrap().as_i64().unwrap(),
        name: json_deck
            .get("name")
            .unwrap()
            .as_str()
            .unwrap()
            .parse()
            .unwrap(),
    })
}
