extern crate clap;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate prettytable;
extern crate rusqlite;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use rusqlite::Connection;
use unicode_segmentation::UnicodeSegmentation;

use persistence::create_table;
use serde_json::{json, to_writer_pretty, Value};

use crate::analysis::{AnalysisQuery, do_extraction_analysis};
use crate::anki_access::{NoteStatus, ZhNote};
use crate::cli_args::get_arg_matches;
use crate::ebook::open_as_book;
use crate::extraction::{extract_vocab, word_to_hanzi, ExtractionItem};
use crate::persistence::{
    add_external_words, insert_overwrite, select_all, select_known, AddedExternal, Vocab,
    VocabStatus,
};
use crate::segmentation::SegmentationMode;
use crate::state::{State, AnalysisState, InfoState, View, ExtractedState};
use crate::tui::enter_tui;
use anyhow::{anyhow, Context, Result};
use std::fs::File;

mod analysis;
mod anki_access;
mod cli_args;
mod ebook;
mod extraction;
mod persistence;
mod segmentation;
mod state;
mod tui;

const DATA_DIR: &str = "/Users/jannes/.zhvocab";
const DATA_PATH: &str = "/Users/jannes/.zhvocab/data.db";
const ANKIDB_PATH: &str = "/Users/jannes/Library/ApplicationSupport/Anki2/Jannes/collection.anki2";
const NOTE_FIELD_PAIRS: [(&str, &str); 1] = [("中文-英文", "中文")];
const WORD_DELIMITERS: [char; 3] = ['/', '\\', ' '];
pub const SUSPENDED_KNOWN_FLAG: i32 = 3;
// green
pub const SUSPENDED_UNKNOWN_FLAG: i32 = 0; // no flag

fn main() -> Result<()> {
    let matches = get_arg_matches();

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
            perform_add_external(&data_conn, filename, AddedExternal::Known)
        }
        Some("add-ignore") => {
            let matches = matches.subcommand_matches("add-ignore").unwrap();
            let filename = matches.value_of("filename").unwrap();
            perform_add_external(&data_conn, filename, AddedExternal::Ignored)
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
        Some("analyze") => {
            let subcommand_matches = matches.subcommand_matches("analyze").unwrap();
            let filename = subcommand_matches.value_of("filename").unwrap();
            let segmentation_mode = if subcommand_matches.is_present("dict-only") {
                SegmentationMode::DictionaryOnly
            } else {
                SegmentationMode::Default
            };
            let known_words: HashSet<String> = select_known(&data_conn)?.into_iter().collect();
            let book = open_as_book(filename)?;
            println!("analyzing book ...");
            let extraction_res = extract_vocab(&book, segmentation_mode);
            let state = State {
                analysis_state: AnalysisState::Extracted(ExtractedState {
                    book,
                    extraction_result: extraction_res,
                    analysis_query: AnalysisQuery {
                        min_occurrence_words: 3,
                        min_occurrence_unknown_chars: None,
                        
                    },
                    analysis_infos: HashMap::new(),
                    known_words,
                }),
                info_state: InfoState::Info,
                current_view: View::Analysis,

            };
            enter_tui(state)
        }
        Some("extract") => {
            let subcommand_matches = matches.subcommand_matches("extract").unwrap();
            let filename = subcommand_matches.value_of("filename").unwrap();
            let min_occurence = subcommand_matches.value_of("min_occurrence").unwrap();
            let min_occ: u64 = min_occurence
                .parse()
                .context("min_occurence must positive number")?;
            if min_occ < 1 {
                return Err(anyhow!("min_occurence must be positive number"));
            }
            let segmentation_mode = if subcommand_matches.is_present("dict-only") {
                SegmentationMode::DictionaryOnly
            } else {
                SegmentationMode::Default
            };
            let known_words: HashSet<String> = select_known(&data_conn)?.into_iter().collect();
            match subcommand_matches.value_of("save as json") {
                Some(outpath) => do_extract(
                    filename,
                    segmentation_mode,
                    min_occ,
                    known_words,
                    Some(outpath),
                ),
                None => do_extract(filename, segmentation_mode, min_occ, known_words, None),
            }
        }
        _ => {
            enter_tui(State::default())
        }
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
    segmentation_mode: SegmentationMode,
    min_occ: u64,
    known_words: HashSet<String>,
    json_outpath: Option<&str>,
) -> Result<()> {
    let book = open_as_book(filename)?;
    println!(
        "extracting vocabulary from {} by {}",
        &book.title, &book.author
    );
    let extraction_res = extract_vocab(&book, segmentation_mode);
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

fn no_subcommand_behavior() -> Result<()> {
    Ok(())
}

fn print_stats(data_conn: &Connection) -> Result<()> {
    let vocabs = select_all(data_conn)?;
    let amount_total_words = &vocabs.len();
    let mut active: HashSet<String> = HashSet::new();
    let mut suspended_known: HashSet<String> = HashSet::new();
    let mut suspended_unknown: HashSet<String> = HashSet::new();
    let mut inactive: HashSet<String> = HashSet::new();
    let mut inactive_ignored: HashSet<String> = HashSet::new();
    for vocab in vocabs {
        match vocab.status {
            VocabStatus::Active => &active.insert(vocab.word),
            VocabStatus::SuspendedKnown => &suspended_known.insert(vocab.word),
            VocabStatus::SuspendedUnknown => &suspended_unknown.insert(vocab.word),
            VocabStatus::AddedExternal(AddedExternal::Known) => &inactive.insert(vocab.word),
            VocabStatus::AddedExternal(AddedExternal::Ignored) => {
                &inactive_ignored.insert(vocab.word)
            }
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
    let amount_total_known = active.len() + suspended_known.len() + inactive.len();

    println!("==========WORDS==========");
    println!("amount total: {}", amount_total_words);
    println!("amount total known: {}", amount_total_known);
    println!("amount active: {}", &active.len());
    println!("amount suspended known: {}", &suspended_known.len());
    println!("amount suspended unknown: {}", &suspended_unknown.len());
    println!("amount inactive known: {}", &inactive.len());
    println!("amount inactive ignored: {}", &inactive_ignored.len());
    println!("==========CHARS==========");
    println!(
        "amount total: {}",
        amount_active_or_know_chars + amount_inactive_chars
    );
    println!("amount active or known: {}", amount_active_or_know_chars);
    println!("amount inactive known: {}", amount_inactive_chars);
    Ok(())
}

fn print_anki_stats() -> Result<()> {
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

fn perform_add_external(data_conn: &Connection, filename: &str, kind: AddedExternal) -> Result<()> {
    let file_str = fs::read_to_string(filename)?;
    let words_to_add: HashSet<String> = file_str
        .split('\n')
        .map(|line| String::from(line.trim()))
        .filter(|trimmed| !trimmed.is_empty())
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
    add_external_words(&data_conn, words_unknown, kind)
}

/// perform first time setup: create sqlite database and words table
/// return database connection
fn first_time_setup() -> Result<Connection> {
    fs::create_dir(DATA_DIR)?;
    let conn = Connection::open(DATA_PATH)?;
    create_table(&conn)?;
    Ok(conn)
}

fn sync_anki_data(data_conn: &Connection) -> Result<()> {
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
