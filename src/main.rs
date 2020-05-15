extern crate rusqlite;
extern crate serde_json;

use rusqlite::{params, Connection, NO_PARAMS};
use serde_json::Value;

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

fn jsondecks_to_decks(json_str: String) -> Vec<Deck> {
    let parsed_json: Value = serde_json::from_str(json_str.as_str()).unwrap();
    let arr = parsed_json.as_object();
    match arr {
        Some(elements) => elements.values().map(jsondeck_to_deck).collect(),
        None => panic!("expected a valid json array of decks"),
    }
}

fn jsondeck_to_deck(json_deck: &Value) -> Deck {
    Deck {
        id: json_deck.get("id").unwrap().as_i64().unwrap(),
        name: json_deck
            .get("name")
            .unwrap()
            .as_str()
            .unwrap()
            .parse()
            .unwrap(),
    }
}

fn main() -> rusqlite::Result<()> {
    let conn = Connection::open("anki-snapshot.db")?;
    let mut example_query = conn.prepare("SELECT decks FROM col")?;
    let mut cols = example_query.query_map(NO_PARAMS, |row| {
        Ok(DecksWrapper {
            json_str: row.get(0)?,
        })
    })?;

    let only_row = cols.next().expect("should have one row");
    println!("{:?}", only_row);

    let decks = jsondecks_to_decks(only_row?.json_str);
    println!("{:?}", decks);

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
