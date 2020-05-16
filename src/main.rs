extern crate rusqlite;
extern crate serde_json;

mod errors;

use errors::AppError;
use rusqlite::{params, Connection, NO_PARAMS};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug)]
struct ExampleQueryResult {
    collection_id: i32,
    last_sync: i32,
}

#[derive(Debug)]
struct DecksWrapper {
    json_str: String,
}

#[derive(Debug)]
struct Deck {
    id: i64,
    name: String,
}

struct ZhNote {
    zh_word: String,
    is_active: bool,
}

#[derive(Debug)]
struct ZhFieldsInfo {
    note_ids: Vec<i64>,
    zh_field_indexes: Vec<i64>,
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
                }
            }
        }
    }
    Ok(ZhFieldsInfo {
        note_ids,
        zh_field_indexes,
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

fn get_decks(conn: &Connection) -> Result<Vec<Deck>, AppError> {
    let mut example_query = conn.prepare("SELECT decks FROM col")?;
    let mut cols = example_query.query_map(NO_PARAMS, |row| {
        Ok(DecksWrapper {
            json_str: row.get(0)?,
        })
    })?;
    let only_row = cols.next().expect("should have one row");
    jsondecks_to_decks(only_row?.json_str).map_err(AppError::from)
}

// fn get_zh_notes(conn: &Connection, decks: &Vec<Deck>) -> Vec<ZhNote> {
//     let mut notes_query = conn.prepare("");
// }

fn main() -> Result<(), AppError> {
    let conn = Connection::open("anki-snapshot.db")?;
    let decks = get_decks(&conn);
    let note_field_map: HashMap<&str, &str> = vec![("中文-英文", "中文")].iter().cloned().collect();
    let zh_notes_info = get_zh_fields_info(&conn, &note_field_map);
    println!("{:?}", decks);
    println!("{:?}", zh_notes_info);
    Ok(())
}

// fn main_mapped() -> () {
//     let cols = Connection::open("anki-snapshot.db")
//         .and_then(|conn| conn.prepare("SELECT id, ls FROM col"))
//         .and_then(|mut query| {
//             query.query_map(NO_PARAMS, |row| {
//                 Ok(ExampleQueryResult {
//                     collection_id: row.get(0)?,
//                     last_sync: row.get(1)?,
//                 })
//             })
//         });
//     match cols {
//         Ok(columns) => {
//             for col in columns {
//                 println!("{:?}", col);
//             }
//         }
//         Err(err) => println!("{:?}", err),
//     }
// }
