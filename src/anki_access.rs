extern crate rusqlite;
extern crate serde_json;

pub use crate::errors::AppError;
use rusqlite::{params, Connection, Statement, NO_PARAMS};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Deck {
    pub id: i64,
    pub name: String,
}

#[derive(Debug)]
pub struct ZhNote {
    pub zh_word: String,
    pub is_active: bool,
}

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
    active_deck_ids: &Vec<i64>,
    non_active_deck_ids: &Vec<i64>,
) -> Result<Vec<ZhNote>, AppError> {
    let fields_info = get_zh_fields_info(conn, note_field_map)?;
    // let mut all_words: Vec<ZhNote> = Vec::new();
    let mut word_map: HashMap<String, bool> = HashMap::new();
    for i in 0..fields_info.amount {
        let note_type_id = *fields_info.note_ids.get(i).unwrap();
        let field_index = *fields_info.zh_field_indexes.get(i).unwrap() as usize;
        let active_deck_ids_str = id_vec_to_sql_set(active_deck_ids);
        let non_active_deck_ids_str = id_vec_to_sql_set(non_active_deck_ids);

        let sql_select_template = "SELECT notes.flds FROM notes JOIN cards \
            ON notes.id = cards.nid \
            WHERE notes.mid = ?1 \
            AND cards.did IN ({})";

        let active_notes_query = conn.prepare(
            sql_select_template
                .replace("{}", &active_deck_ids_str)
                .as_str(),
        )?;
        let active_words: Vec<String> =
            select_note_sql_to_result(active_notes_query, note_type_id, field_index)?;
        for word in active_words {
            word_map.insert(word, true);
        }

        let non_active_notes_query = conn.prepare(
            sql_select_template
                .replace("{}", &non_active_deck_ids_str)
                .as_str(),
        )?;
        let non_active_words: Vec<String> =
            select_note_sql_to_result(non_active_notes_query, note_type_id, field_index)?;
        // only mark words as inactive if they haven't been marked active before:
        // all cards of a note need to be inactive for the note to count as inactive
        for word in non_active_words {
            if !word_map.contains_key(&word) {
                word_map.insert(word, false);
            }
        }
    }
    Ok(word_map
        .into_iter()
        .map(|(zh_word, is_active)| ZhNote { zh_word, is_active })
        .collect())
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

fn select_note_sql_to_result(
    mut stmt: Statement,
    note_type_id: i64,
    field_index: usize,
) -> Result<Vec<String>, rusqlite::Error> {
    stmt.query_map(params![note_type_id], |row| {
        let fields_str: String = row.get(0)?;
        Ok(fieldsstr_to_field(fields_str.as_str(), field_index))
    })?
    .collect::<Result<Vec<String>, _>>()
}

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
                    let node_id_str = note.get("id").unwrap().as_str().unwrap();
                    let note_id_int: i64 = node_id_str.parse().unwrap();
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

fn jsondecks_to_decks(json_str: String) -> Result<Vec<Deck>, serde_json::Error> {
    let parsed_json: Value = serde_json::from_str(json_str.as_str())?;
    let arr = parsed_json.as_object();
    match arr {
        Some(elements) => elements.values().map(jsondeck_to_deck).collect(),
        None => panic!("expected a valid json array of decks"),
    }
}

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
