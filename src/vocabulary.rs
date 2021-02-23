use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use crate::persistence::{select_all, AddedExternal, VocabStatus};
use anyhow::Result;
use rusqlite::Connection;
use unicode_segmentation::UnicodeSegmentation;

pub struct VocabularyInfo {
    pub words_total: usize,
    pub words_total_known: usize,
    pub words_active: usize,
    pub words_suspended_unknown: usize,
    pub words_suspended_known: usize,
    pub words_inactive_known: usize,
    pub words_inactive_ignored: usize,
    pub chars_total_known: usize,
    pub chars_active_or_suspended_known: usize,
    pub chars_inactive_known: usize,
}

impl VocabularyInfo {
    pub fn words_description(&self) -> Vec<String> {
        vec![
            format!("words total known: {}", self.words_total_known),
            format!("words active: {}", self.words_active),
            format!("words suspended unknown: {}", self.words_suspended_unknown),
            format!("words suspended known: {}", self.words_suspended_known),
            format!("words inactive known: {}", self.words_inactive_known),
            format!("words inactive ignored: {}", self.words_inactive_ignored),
        ]
    }

    pub fn chars_description(&self) -> Vec<String> {
        vec![
            format!("chars total known: {}", self.chars_total_known),
            format!("chars active / suspended known: {}", self.chars_active_or_suspended_known),
            format!("chars inactive known: {}", self.chars_inactive_known),
        ]
    }
}

pub fn get_vocab_stats(data_conn: Arc<Mutex<Connection>>) -> Result<VocabularyInfo> {
    let vocabs = select_all(&data_conn.lock().unwrap())?;
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

    Ok(VocabularyInfo {
        words_total: *amount_total_words,
        words_total_known: amount_total_known,
        words_active: active.len(),
        words_suspended_unknown: suspended_unknown.len(),
        words_suspended_known: suspended_known.len(),
        words_inactive_known: inactive.len(),
        words_inactive_ignored: inactive_ignored.len(),
        chars_total_known: amount_active_or_know_chars + amount_inactive_chars,
        chars_active_or_suspended_known: *amount_active_or_know_chars,
        chars_inactive_known: *amount_inactive_chars,
    })
}
