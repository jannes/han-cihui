extern crate clap;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate prettytable;
extern crate rusqlite;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use clap::{App, Arg, SubCommand};
use prettytable::Table;
use rusqlite::Connection;
use unicode_segmentation::UnicodeSegmentation;

use errors::AppError;
use persistence::create_table;
use serde_json::{json, to_writer_pretty, Value};

use crate::anki_access::{NoteStatus, ZhNote};
use crate::ebook::open_as_book;
use crate::errors::AppError::InvalidCLIArgument;
use crate::extraction::{extract_vocab, word_to_hanzi, ExtractionItem, ExtractionResult, Pkuseg};
use crate::persistence::{
    add_external_words, insert_overwrite, select_all, select_known, Vocab, VocabStatus,
};
use std::fs::File;

mod anki_access;
mod ebook;
mod errors;
mod extraction;
mod persistence;
mod python_interop;

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
        .subcommand(
            SubCommand::with_name("extract")
                .about("extract vocabulary from epub")
                .arg(
                    Arg::with_name("filename")
                        .required(true)
                        .help("path to epub file, from which to extract vocabulary"),
                )
                .arg(
                    Arg::with_name("min_occurrence")
                        .required(true)
                        .help("the minimum amount a word should occur to be extracted"),
                )
                .arg(Arg::with_name("save as json")
                    .required(false)
                    .long("save-json")
                    .takes_value(true)
                    .help("save words with minimum occurrence as json array with per chapter vocab"),
                )
        )
        .subcommand(SubCommand::with_name("show").about("prints all known words"))
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
        Some("show") => {
            let known_words = select_known(&data_conn)?;
            for word in known_words {
                println!("{}", word);
            }
            Ok(())
        }
        Some("extract") => {
            let subcommand_matches = matches.subcommand_matches("extract").unwrap();
            let filename = subcommand_matches.value_of("filename").unwrap();
            let min_occurence = subcommand_matches.value_of("min_occurrence").unwrap();
            let min_occ: u64 = min_occurence.parse().map_err(|_e| {
                InvalidCLIArgument("min_occurence must positive number".to_string())
            })?;
            if min_occ < 1 {
                return Err(AppError::InvalidCLIArgument(
                    "min_occurence must be positive number".to_string(),
                ));
            }
            let known_words: HashSet<String> = select_known(&data_conn)?.into_iter().collect();
            match subcommand_matches.value_of("save as json") {
                Some(outpath) => do_extract(filename, min_occ, known_words, Some(outpath)),
                None => do_extract(filename, min_occ, known_words, None),
            }
        }
        _ => no_subcommand_behavior(),
    }
}

fn ext_item_set_to_char_freq(ext_items: &HashSet<&ExtractionItem>) -> HashMap<String, u64> {
    let mut char_freq_map: HashMap<String, u64> = HashMap::new();
    ext_items
        .iter()
        .map(|item| (word_to_hanzi(&item.word), item.frequency))
        .for_each(|(hanzis, frequency)| {
            for hanzi in hanzis {
                if char_freq_map.contains_key(hanzi) {
                    let v = char_freq_map.get_mut(hanzi).unwrap();
                    *v += frequency;
                } else {
                    char_freq_map.insert(hanzi.to_string(), frequency);
                }
            }
        });
    char_freq_map
}

fn do_extract(
    filename: &str,
    min_occ: u64,
    known_words: HashSet<String>,
    json_outpath: Option<&str>,
) -> Result<(), AppError> {
    let book = open_as_book(filename)?;
    println!(
        "extracting vocabulary from {} by {}",
        &book.title, &book.author
    );
    let extraction_res = extract_vocab(&book, &Pkuseg {});
    let filtered_extraction_set = do_extraction_analysis(&extraction_res, min_occ, known_words);
    if let Some(outpath) = json_outpath {
        let chapter_titles: Vec<String> = book
            .chapters
            .iter()
            .map(|chapter| chapter.get_numbered_title())
            .collect();
        let mut chapter_vocabulary: HashMap<&str, HashSet<&ExtractionItem>> = chapter_titles
            .iter()
            .map(|chapter_title| (chapter_title.as_str(), HashSet::new()))
            .collect();
        for item in filtered_extraction_set {
            chapter_vocabulary
                .get_mut(item.location.as_str())
                .unwrap()
                .insert(item);
        }
        let chapter_jsons: Vec<Value> = chapter_titles
            .iter()
            .map(|chapter_title| {
                json!({
                "title": chapter_title,
                "words": chapter_vocabulary.get(chapter_title.as_str()).unwrap().iter()
                .map(|item| item.word.as_str()).collect::<Vec<&str>>()
                })
            })
            .collect();
        let output_json = json!({
            "title": &book.title,
            "vocabulary": chapter_jsons
        });
        to_writer_pretty(&File::create(outpath)?, &output_json)?;
    }

    Ok(())
}

fn do_extraction_analysis(
    extraction_res: &ExtractionResult,
    min_occ: u64,
    known_words: HashSet<String>,
) -> HashSet<&ExtractionItem> {
    let known_chars: HashSet<&str> = known_words
        .iter()
        .flat_map(|word| word_to_hanzi(&word))
        .collect();
    /* ALL WORDS */
    let amount_unique_words = extraction_res.vocabulary_info.len();
    let amount_unique_chars = extraction_res.char_freq_map.len();
    let unknown_voc: HashSet<&ExtractionItem> = extraction_res
        .vocabulary_info
        .iter()
        .filter(|item| !known_words.contains(&item.word))
        .collect();
    let unknown_char: HashMap<&str, u64> = extraction_res
        .char_freq_map
        .iter()
        .map(|(k, v)| (k.as_str(), *v))
        .filter(|(k, _v)| !known_chars.contains(k))
        .collect();
    let amount_unknown_words: u64 = unknown_voc.iter().map(|item| item.frequency).sum();
    let amount_unknown_chars: u64 = unknown_char.iter().map(|(_k, v)| v).sum();

    /* MIN OCCURRING WORDS */
    let vocabulary_min_occurring: HashSet<&ExtractionItem> = extraction_res
        .vocabulary_info
        .iter()
        .filter(|item| item.frequency >= min_occ)
        .collect();
    let total_min_occurring_words: u64 = vocabulary_min_occurring
        .iter()
        .map(|item| item.frequency)
        .sum();

    let char_freq_min_occur: HashMap<String, u64> =
        ext_item_set_to_char_freq(&vocabulary_min_occurring);
    let total_char_min_occur: u64 = char_freq_min_occur.iter().map(|(_char, freq)| freq).sum();

    let unknown_voc_min_occ: HashSet<&ExtractionItem> = vocabulary_min_occurring
        .iter()
        .copied()
        .filter(|item| !known_words.contains(&item.word))
        .collect();
    let total_unknown_min_occur_words: u64 =
        unknown_voc_min_occ.iter().map(|item| item.frequency).sum();

    let unknown_char_min_occur: HashMap<&String, u64> = char_freq_min_occur
        .iter()
        .filter(|(hanzi, _freq)| !known_chars.contains(hanzi.as_str()))
        .map(|(hanzi, freq)| (hanzi, *freq))
        .collect();

    let total_unknown_char_min_occur: u64 = unknown_char_min_occur
        .iter()
        .map(|(_char, freq)| freq)
        .sum();

    let mut table = Table::new();
    table.add_row(row!["", "all", format!("min {}", min_occ)]);
    table.add_row(row![
        "total amount words",
        extraction_res.word_count,
        total_min_occurring_words
    ]);
    table.add_row(row![
        "total amount unknown words",
        amount_unknown_words,
        total_unknown_min_occur_words
    ]);
    table.add_row(row![
        "total amount characters",
        extraction_res.character_count,
        total_char_min_occur
    ]);
    table.add_row(row![
        "total amount unknown characters",
        amount_unknown_chars,
        total_unknown_char_min_occur
    ]);
    table.add_row(row![
        "amount unique words",
        amount_unique_words,
        vocabulary_min_occurring.len()
    ]);
    table.add_row(row![
        "amount unknown unique words",
        unknown_voc.len(),
        unknown_voc_min_occ.len()
    ]);
    table.add_row(row![
        "amount unique characters",
        amount_unique_chars,
        char_freq_min_occur.len()
    ]);
    table.add_row(row![
        "amount unknown unique characters",
        unknown_char.len(),
        unknown_char_min_occur.len()
    ]);
    table.printstd();
    unknown_voc_min_occ
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
    let mut active_or_known_characters: HashSet<&str> = HashSet::new();
    let mut inactive_characters: HashSet<&str> = HashSet::new();
    for word in &active.union(&suspended_known).collect::<Vec<&String>>() {
        let chars: Vec<&str> = UnicodeSegmentation::graphemes(word.as_str(), true).collect();
        for char in chars {
            active_or_known_characters.insert(char);
        }
    }
    for word in &inactive.union(&suspended_unknown).collect::<Vec<&String>>() {
        let chars: Vec<&str> = UnicodeSegmentation::graphemes(word.as_str(), true).collect();
        for char in chars {
            if !active_or_known_characters.contains(char) {
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
        .split('\n')
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
        .map(String::from)
        .collect()
}
