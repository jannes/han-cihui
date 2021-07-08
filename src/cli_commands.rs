use std::collections::HashSet;

use anyhow::Result;
use clap::ArgMatches;
use rusqlite::Connection;
use unicode_segmentation::UnicodeSegmentation;

use crate::persistence::{select_all, select_by_status, select_known, AddedExternal, VocabStatus};

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
        select_known(&conn)
    }?;
    let target_items = if matches.is_present("kind") {
        let kind = matches.value_of("kind").unwrap();
        match kind {
            "words" => target_words,
            "chars" => {
                let vocabs = select_all(&conn)?;
                let mut active: HashSet<String> = HashSet::new();
                let mut suspended_known: HashSet<String> = HashSet::new();
                let mut inactive: HashSet<String> = HashSet::new();
                for vocab in vocabs {
                    match vocab.status {
                        VocabStatus::Active => {
                            active.insert(vocab.word);
                        }
                        VocabStatus::SuspendedKnown => {
                            suspended_known.insert(vocab.word);
                        }
                        VocabStatus::AddedExternal(AddedExternal::Known) => {
                            inactive.insert(vocab.word);
                        }
                        _ => {}
                    };
                }
                let mut active_or_known_characters: HashSet<&str> = HashSet::new();
                let mut inactive_characters: HashSet<String> = HashSet::new();
                for word in &active.union(&suspended_known).collect::<Vec<&String>>() {
                    let chars: Vec<&str> =
                        UnicodeSegmentation::graphemes(word.as_str(), true).collect();
                    for char in chars {
                        active_or_known_characters.insert(char);
                    }
                }
                for word in &inactive {
                    let chars: Vec<&str> =
                        UnicodeSegmentation::graphemes(word.as_str(), true).collect();
                    for char in chars {
                        if !active_or_known_characters.contains(char) {
                            inactive_characters.insert(char.to_string());
                        }
                    }
                }
                inactive_characters
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