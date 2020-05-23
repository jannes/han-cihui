extern crate clap;
extern crate rusqlite;

use std::collections::{HashMap, HashSet};

use rusqlite::{params, Connection, NO_PARAMS};

use crate::anki_access::{NoteStatus, ZhNote};
use crate::persistence::{add_external_words, insert_overwrite, select_all, Vocab, VocabStatus};
use clap::{App, Arg, SubCommand};
use errors::AppError;
use persistence::create_table;
use std::fs;
use std::path::Path;

mod anki_access;
mod errors;
mod persistence;

const DATA_DIR: &str = "/Users/jannes/.zhvocab";
const DATA_PATH: &str = "/Users/jannes/.zhvocab/data.db";
const ANKIDB_PATH: &str = "/Users/jannes/Library/ApplicationSupport/Anki2/Jannes/collection.anki2";
const NOTE_FIELD_PAIRS: [(&str, &str); 1] = [("中文-英文", "中文")];
const WORD_DELIMITERS: [char; 3] = ['/', '\\', ' '];
pub const SUSPENDED_KNOWN_FLAG: i32 = 3; // green
pub const SUSPENDED_UNKNOWN_FLAG: i32 = 0; // no flag

fn main() -> Result<(), AppError> {
    let matches = App::new("中文 vocab")
        .version("0.1")
        .subcommand(
            SubCommand::with_name("add")
                .about("add vocabulary from file")
                .arg(
                    Arg::with_name("filename")
                        .required(true)
                        .help("path to file with one word per line"),
                ),
        )
        .subcommand(SubCommand::with_name("sync").about("syncs data with Anki"))
        .subcommand(
            SubCommand::with_name("stats")
                .about("print vocabulary statistiscs")
                .arg(
                    Arg::with_name("anki only")
                        .required(false)
                        .short("a")
                        .long("anki")
                        .help("print anki statistics only"),
                ),
        )
        .get_matches();

    let data_conn: Connection;
    // if first time call, do data setup
    if !Path::new(DATA_PATH).exists() {
        println!("performing first time setup");
        data_conn = first_time_setup()?;
    } else {
        data_conn = Connection::open(DATA_PATH)?;
    }

    match matches.subcommand_name() {
        Some("add") => {
            let matches = matches.subcommand_matches("add").unwrap();
            let filename = matches.value_of("filename").unwrap();
            perform_add_external(&data_conn, filename)
        }
        Some("sync") => {
            println!("syncing Anki data");
            sync_anki_data(&data_conn)?;
            println!("done");
            Ok(())
        }
        Some("stats") => {
            let matches = matches.subcommand_matches("stats").unwrap();
            if matches.is_present("anki only") {
                print_anki_stats()
            } else {
                print_stats(&data_conn)
            }
        }
        _ => no_subcommand_behavior(),
    }
}

fn no_subcommand_behavior() -> Result<(), AppError> {
    Ok(())
}

fn print_stats(data_conn: &Connection) -> Result<(), AppError> {
    let vocabs = select_all(data_conn)?;
    let amount_total_words = &vocabs.len();
    let mut active: HashSet<String> = HashSet::new();
    let mut suspended_known: HashSet<String> = HashSet::new();
    let mut suspended_unknown: HashSet<String> = HashSet::new();
    let mut inactive: HashSet<String> = HashSet::new();
    for vocab in vocabs {
        match vocab.status {
            VocabStatus::Active => &active.insert(vocab.word),
            VocabStatus::SuspendedKnown => &suspended_known.insert(vocab.word),
            VocabStatus::SuspendedUnknown => &suspended_unknown.insert(vocab.word),
            VocabStatus::AddedExternal => &inactive.insert(vocab.word),
        };
    }
    let mut active_or_known_characters = HashSet::new();
    let mut inactive_characters = HashSet::new();
    for word in &active.union(&suspended_known).collect::<Vec<&String>>() {
        for char in word.chars() {
            active_or_known_characters.insert(char);
        }
    }
    for word in &inactive.union(&suspended_unknown).collect::<Vec<&String>>() {
        for char in word.chars() {
            if !active_or_known_characters.contains(&char) {
                inactive_characters.insert(char);
            }
        }
    }
    let amount_active_or_know_chars = &active_or_known_characters.len();
    let amount_inactive_chars = &inactive_characters.len();

    println!("==========WORDS==========");
    println!("amount total: {}", amount_total_words);
    println!("amount active: {}", &active.len());
    println!("amount suspended known: {}", &suspended_known.len());
    println!("amount suspended unknown: {}", &suspended_unknown.len());
    println!("amount inactive: {}", &inactive.len());
    println!("==========CHARS==========");
    println!(
        "amount total: {}",
        amount_active_or_know_chars + amount_inactive_chars
    );
    println!("amount active or known: {}", amount_active_or_know_chars);
    println!("amount inactive: {}", amount_inactive_chars);
    Ok(())
}

fn print_anki_stats() -> Result<(), AppError> {
    let conn = Connection::open(ANKIDB_PATH)?;
    let note_field_map: HashMap<&str, &str> = NOTE_FIELD_PAIRS.iter().cloned().collect();
    let zh_notes = anki_access::get_zh_notes(&conn, &note_field_map)?;

    let active_words = notes_to_words_filtered(&zh_notes, NoteStatus::Active);
    let inactive_unknown_words = notes_to_words_filtered(&zh_notes, NoteStatus::SuspendedUnknown);
    let inactive_known_words = notes_to_words_filtered(&zh_notes, NoteStatus::SuspendedKnown);

    let active_chars: HashSet<char> = active_words.iter().flat_map(|word| word.chars()).collect();

    println!("total notes: {}", zh_notes.len());
    println!("active: {}", active_words.len());
    println!("active characters: {}", active_chars.len());
    println!("inactive known: {}", inactive_known_words.len());
    println!("inactive unknown: {}", inactive_unknown_words.len());

    for word in &inactive_unknown_words {
        println!("{}", word);
    }
    Ok(())
}

fn perform_add_external(data_conn: &Connection, filename: &str) -> Result<(), AppError> {
    let file_str = fs::read_to_string(filename)?;
    let words_to_add: HashSet<String> = file_str
        .split("\n")
        .map(|line| String::from(line.trim()))
        .collect();
    let words_known: HashSet<String> = select_all(data_conn)?
        .iter()
        .map(|vocab| String::from(&vocab.word))
        .collect();
    let words_unknown: &HashSet<&str> = &words_to_add
        .difference(&words_known)
        .map(|s| s.as_str())
        .collect();
    println!("amount saved: {}", &words_known.len());
    println!("amount to add: {}", &words_to_add.len());
    println!("amount new: {}", &words_unknown.len());
    add_external_words(&data_conn, words_unknown)
}

/// perform first time setup: create sqlite database and words table
/// return database connection
fn first_time_setup() -> Result<Connection, AppError> {
    fs::create_dir(DATA_DIR)?;
    let conn = Connection::open(DATA_PATH)?;
    create_table(&conn)?;
    Ok(conn)
}

fn sync_anki_data(data_conn: &Connection) -> Result<(), AppError> {
    let conn = Connection::open(ANKIDB_PATH)?;
    let note_field_map: HashMap<&str, &str> = NOTE_FIELD_PAIRS.iter().cloned().collect();
    let zh_notes = anki_access::get_zh_notes(&conn, &note_field_map)?;
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

fn notes_to_words_filtered(notes: &HashSet<ZhNote>, status: NoteStatus) -> HashSet<String> {
    notes
        .iter()
        .filter(|note| note.status == status)
        .flat_map(|note| zh_field_to_words(&note.zh_field))
        .collect()
}

fn zh_field_to_words(field: &str) -> Vec<String> {
    field
        .split(&WORD_DELIMITERS[..])
        .map(|s| String::from(s))
        .collect()
}
