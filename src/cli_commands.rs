use std::{
    collections::{HashMap, HashSet},
    fs,
};

use anyhow::Result;
use clap::ArgMatches;
use rusqlite::Connection;
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    anki_access::{self, NoteStatus, ZhNote},
    persistence::{
        add_external_words, delete_words, select_all, select_by_status, select_known,
        AddedExternal, VocabStatus,
    },
    NOTE_FIELD_PAIRS, WORD_DELIMITERS,
};

pub fn show(matches: &ArgMatches, conn: &Connection) -> Result<()> {
    let target_words = if matches.is_present("status") {
        let status = matches.value_of("status").unwrap();
        match status {
            "known_external" => {
                select_by_status(conn, VocabStatus::AddedExternal(AddedExternal::Known))
            }
            "unknown_suspended" => select_by_status(conn, VocabStatus::SuspendedUnknown),
            _ => panic!("unknown value for vocabulary status"),
        }
    } else {
        select_known(conn)
    }?;
    let target_items = if matches.is_present("kind") {
        let kind = matches.value_of("kind").unwrap();
        match kind {
            "words" => target_words,
            "chars" => {
                let vocabs = select_all(conn)?;
                let mut active_or_known: HashSet<String> = HashSet::new();
                let mut active_or_known_characters: HashSet<&str> = HashSet::new();
                let mut target_characters: HashSet<String> = HashSet::new();
                for vocab in vocabs {
                    match vocab.status {
                        VocabStatus::Active => {
                            active_or_known.insert(vocab.word);
                        }
                        VocabStatus::SuspendedKnown => {
                            active_or_known.insert(vocab.word);
                        }
                        _ => {}
                    };
                }
                for word in &active_or_known {
                    let chars: Vec<&str> =
                        UnicodeSegmentation::graphemes(word.as_str(), true).collect();
                    for char in chars {
                        active_or_known_characters.insert(char);
                    }
                }
                // include only characters that are neither active nor guaranteed known
                for word in &target_words {
                    let chars: Vec<&str> =
                        UnicodeSegmentation::graphemes(word.as_str(), true).collect();
                    for char in chars {
                        if !active_or_known_characters.contains(char) {
                            target_characters.insert(char.to_string());
                        }
                    }
                }
                target_characters
            }
            _ => panic!("invalid vocab kind, expected 'words' or 'chars'"),
        }
    } else {
        target_words
    };
    for item in target_items {
        println!("{}", item);
    }
    Ok(())
}

pub fn print_anki_stats(conn: &Connection) -> Result<()> {
    let note_field_map: HashMap<&str, &str> = NOTE_FIELD_PAIRS.iter().cloned().collect();
    let zh_notes = anki_access::get_zh_notes(conn, &note_field_map)?;

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

pub fn perform_add_external(
    data_conn: &Connection,
    filename: &str,
    kind: AddedExternal,
) -> Result<()> {
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
    add_external_words(data_conn, words_unknown, kind)
}

pub fn perform_delete_external(data_conn: &Connection, filename: &str) -> Result<()> {
    let file_str = fs::read_to_string(filename)?;
    let words_to_delete: HashSet<String> = file_str
        .split('\n')
        .map(|line| String::from(line.trim()))
        .filter(|trimmed| !trimmed.is_empty())
        .collect();
    println!("amount to delete: {}", &words_to_delete.len());
    delete_words(data_conn, &words_to_delete)
}

pub fn zh_field_to_words(field: &str) -> Vec<String> {
    field
        .split(&WORD_DELIMITERS[..])
        .map(String::from)
        .collect()
}

fn notes_to_words_filtered(notes: &HashSet<ZhNote>, status: NoteStatus) -> HashSet<String> {
    notes
        .iter()
        .filter(|note| note.status == status)
        .flat_map(|note| zh_field_to_words(&note.zh_field))
        .collect()
}
