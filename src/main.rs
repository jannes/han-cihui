extern crate rusqlite;

use std::collections::{HashMap, HashSet};

use rusqlite::{params, Connection, NO_PARAMS};

use errors::AppError;

mod anki_access;
mod errors;

const ANKIDB_PATH: &str = "/Users/jannes/Library/ApplicationSupport/Anki2/Jannes/collection.anki2";
const ACTIVE_DECK: &str = "中文";
const NON_ACTIVE_DECK: &str = "中文-inactive";
const NOTE_FIELD_PAIRS: [(&str, &str); 1] = [("中文-英文", "中文")];
const WORD_DELIMITERS: [char; 3] = ['/', '\\', ' '];

fn main() -> Result<(), AppError> {
    let conn = Connection::open(ANKIDB_PATH)?;
    let decks = anki_access::get_decks(&conn)?;
    let mut active_deck_ids = Vec::new();
    let mut non_active_deck_ids = Vec::new();
    for deck in decks {
        let split: Vec<&str> = deck.name.split("::").collect();
        if split.contains(&ACTIVE_DECK) {
            active_deck_ids.push(deck.id);
        } else if split.contains(&NON_ACTIVE_DECK) {
            non_active_deck_ids.push(deck.id);
        }
    }
    println!("active deck ids: {:?}", active_deck_ids);
    println!("non-active deck ids: {:?}", non_active_deck_ids);

    let note_field_map: HashMap<&str, &str> = NOTE_FIELD_PAIRS.iter().cloned().collect();
    // let note_field_map: HashMap<&str, &str> = vec![("中文-英文", "中文")].iter().cloned().collect();
    let zh_notes = anki_access::get_zh_notes(
        &conn,
        &note_field_map,
        &active_deck_ids,
        &non_active_deck_ids,
    )?;

    let active_words: HashSet<String> = zh_notes
        .iter()
        .filter(|note| note.is_active)
        .flat_map(|note| zh_field_to_words(&note.zh_word))
        .collect();

    let active_chars: HashSet<char> = active_words.iter().flat_map(|word| word.chars()).collect();

    let non_active_words: HashSet<String> = zh_notes
        .iter()
        .filter(|note| !note.is_active)
        .flat_map(|note| zh_field_to_words(&note.zh_word))
        .collect();

    println!("total notes: {}", zh_notes.len());
    println!("active: {}", active_words.len());
    println!("active characters: {}", active_chars.len());
    println!("inactive: {}", non_active_words.len());

    for word in &non_active_words {
        println!("{}", word);
    }

    Ok(())
}

fn zh_field_to_words(field: &str) -> Vec<String> {
    field
        .split(&WORD_DELIMITERS[..])
        .map(|s| String::from(s))
        .collect()
}

fn word_to_chars(word: &str) -> HashSet<char> {
    word.chars().collect()
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
