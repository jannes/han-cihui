use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};

use crate::segmentation::{segment_text, BookSegmentation, ChapterSegmentation};
use jieba_rs::Jieba;
use lazy_static::lazy_static;
use regex::Regex;

use unicode_segmentation::UnicodeSegmentation;

#[derive(PartialEq, Eq, Hash)]
pub struct ExtractionItem {
    pub word: String,
    pub frequency: u64,
    pub first_location: String,
}

pub struct ExtractionResult {
    pub segmented_book: BookSegmentation,
    pub vocabulary: HashSet<ExtractionItem>,
}

/// Extract all words from given text
/// returns words in simplified form
pub fn extract_words(
    text: &str,
    jieba: &Jieba,
    mapping_fan2jian: &HashMap<String, String>,
    mapping_jian2fan: &HashMap<String, String>,
) -> HashSet<String> {
    let segmented = segment_text(text, jieba, mapping_fan2jian, mapping_jian2fan);
    segmented.into_iter().collect()
}

/// Computes extraction result from a segmented book
pub fn extract_vocab_from_segmented(segmented_book: BookSegmentation) -> ExtractionResult {
    if segmented_book.chapter_cuts.is_empty() {
        panic!("expected book with at least one chapter!");
    }
    let mut word_frequencies: HashMap<String, u64> = HashMap::new();
    let mut word_occurrences: HashMap<String, String> = HashMap::new();
    for ChapterSegmentation { title, cut } in segmented_book.chapter_cuts.clone() {
        update_word_info(cut, title, &mut word_frequencies, &mut word_occurrences);
    }
    let vocabulary = word_occurrences
        .into_iter()
        .filter(|(word, _)| contains_hanzi(word))
        .map(|(word, chapter)| ExtractionItem {
            word: word.to_string(),
            frequency: *word_frequencies.get(&word).unwrap(),
            first_location: chapter.to_string(),
        })
        .collect();
    ExtractionResult {
        segmented_book,
        vocabulary,
    }
}

pub fn contains_hanzi(word: &str) -> bool {
    lazy_static! {
        static ref HAN_RE: Regex = Regex::new(r"\p{Han}").unwrap();
    }
    HAN_RE.is_match(word)
}

pub fn word_to_hanzi(word: &str) -> Vec<&str> {
    UnicodeSegmentation::graphemes(word, true).collect::<Vec<&str>>()
}

fn update_word_info(
    words: Vec<String>,
    chapter_title: String,
    word_frequencies: &mut HashMap<String, u64>,
    word_occurrences: &mut HashMap<String, String>,
) {
    for word in words {
        match word_frequencies.entry(word.clone()) {
            Entry::Occupied(o) => {
                *o.into_mut() += 1;
            }
            Entry::Vacant(v) => {
                v.insert(1);
                word_occurrences.insert(word, chapter_title.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::extraction::contains_hanzi;

    #[test]
    fn match_hanzi_words() {
        let hello = "你好";
        let name = "思明";
        let mixed = "i am 诗文";
        let english = "dance baby";
        let punctuation = "。，、……";
        assert!(contains_hanzi(hello));
        assert!(contains_hanzi(name));
        assert!(contains_hanzi(mixed));
        assert!(!contains_hanzi(english));
        assert!(!contains_hanzi(punctuation));
    }
}
