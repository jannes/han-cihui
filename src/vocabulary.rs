use std::collections::HashSet;

use crate::{
    db::vocab::{db_words_select_all, VocabStatus},
    extraction::word_to_hanzi,
};
use anyhow::Result;
use rusqlite::Connection;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy)]
pub struct VocabularyInfo {
    pub words_known: usize,
    pub words_active: usize,
    pub words_inactive: usize,
    pub chars_known: usize,
    pub chars_active: usize,
    pub chars_inactive: usize,
}

impl VocabularyInfo {
    pub fn words_description(&self) -> Vec<String> {
        vec![
            format!("words total known: {}", self.words_known),
            format!("words active: {}", self.words_active),
            format!("words inactive {}", self.words_inactive),
        ]
    }

    pub fn chars_description(&self) -> Vec<String> {
        vec![
            format!("chars total known: {}", self.chars_known),
            format!("chars active: {}", self.chars_active),
            format!("chars inactive: {}", self.chars_inactive),
        ]
    }
}

pub fn get_known_chars(known_words: &HashSet<String>) -> HashSet<String> {
    known_words
        .iter()
        .flat_map(|w| word_to_hanzi(w))
        .map(|hanzi| hanzi.to_string())
        .collect()
}

pub fn get_known_words_and_chars(known_words: HashSet<String>) -> HashSet<String> {
    known_words
        .union(&get_known_chars(&known_words))
        .map(|s| s.to_string())
        .collect::<HashSet<String>>()
}

pub fn get_vocab_stats(data_conn: &Connection) -> Result<VocabularyInfo> {
    let vocabs = db_words_select_all(data_conn)?;

    let mut words_active: HashSet<String> = HashSet::new();
    let mut words_inactive: HashSet<String> = HashSet::new();
    let mut words_external: HashSet<String> = HashSet::new();
    for (word, status) in vocabs {
        match status {
            VocabStatus::Active => &words_active.insert(word),
            VocabStatus::Inactive => &words_inactive.insert(word),
            VocabStatus::AddedExternal => &words_external.insert(word),
        };
    }

    let mut chars_active: HashSet<&str> = HashSet::new();
    let mut chars_inactive: HashSet<&str> = HashSet::new();
    let mut chars_external: HashSet<&str> = HashSet::new();

    for word in &words_active {
        let chars: Vec<&str> = UnicodeSegmentation::graphemes(word.as_str(), true).collect();
        for char in chars {
            chars_active.insert(char);
        }
    }
    for word in &words_external {
        let chars: Vec<&str> = UnicodeSegmentation::graphemes(word.as_str(), true).collect();
        for char in chars {
            if !chars_active.contains(char) {
                chars_external.insert(char);
            }
        }
    }
    for word in &words_inactive {
        let chars: Vec<&str> = UnicodeSegmentation::graphemes(word.as_str(), true).collect();
        for char in chars {
            if !chars_active.contains(char) && !chars_external.contains(char) {
                chars_inactive.insert(char);
            }
        }
    }

    Ok(VocabularyInfo {
        words_known: words_active.len() + words_external.len(),
        words_active: words_active.len(),
        words_inactive: words_inactive.len(),
        chars_known: chars_active.len() + chars_external.len(),
        chars_active: chars_active.len(),
        chars_inactive: chars_inactive.len(),
    })
}
