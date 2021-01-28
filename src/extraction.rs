use std::collections::{HashMap, HashSet};

use crate::ebook::{Book, Chapter};
use crate::segmentation::{segment_book, SegmentationMode};
use regex::Regex;
use unicode_segmentation::UnicodeSegmentation;

#[derive(PartialEq, Eq, Hash)]
pub struct ExtractionItem {
    pub(crate) word: String,
    pub(crate) frequency: u64,
    pub(crate) location: String,
}

pub struct ExtractionResult {
    pub(crate) word_count: u64,
    pub(crate) character_count: u64,
    pub(crate) vocabulary_info: HashSet<ExtractionItem>,
    // char is always 4 bytes unicode scalar, not necessarily an actual full character
    pub(crate) char_freq_map: HashMap<String, u64>,
}

pub fn extract_vocab(book: &Book, segmentation_mode: SegmentationMode) -> ExtractionResult {
    let word_occur_freq = extract(book, segmentation_mode);
    let mut word_count: u64 = 0;
    let mut character_count: u64 = 0;
    let mut vocabulary_info: HashSet<ExtractionItem> = HashSet::new();
    let mut char_freq_map: HashMap<String, u64> = HashMap::new();

    for (word, (chapter, frequency)) in word_occur_freq {
        if contains_hanzi(&word) {
            word_count += frequency;
            let characters = word_to_hanzi(&word);
            for character in characters {
                character_count += frequency;
                if char_freq_map.contains_key(character) {
                    let v = char_freq_map.get_mut(character).unwrap();
                    *v += frequency;
                } else {
                    char_freq_map.insert(character.to_string(), frequency);
                }
            }
            let extraction_item = ExtractionItem {
                word,
                frequency,
                location: chapter.get_numbered_title(),
            };
            vocabulary_info.insert(extraction_item);
        }
    }

    ExtractionResult {
        word_count,
        character_count,
        vocabulary_info,
        char_freq_map,
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

fn update_word_info<'a, 'b>(
    words: Vec<&'a str>,
    current_chapter: &'b Chapter,
    word_frequencies: &mut HashMap<&'a str, u64>,
    word_occurrences: &mut HashMap<&'a str, &'b Chapter>,
) {
    for word in words {
        let mut frequency = 1;
        if word_frequencies.contains_key(word) {
            frequency += word_frequencies.get(word).unwrap();
        } else {
            word_occurrences.insert(word, current_chapter);
        }
        word_frequencies.insert(word, frequency);
    }
}

fn extract(book: &Book, segmentation_mode: SegmentationMode) -> HashMap<String, (&Chapter, u64)> {
    if book.chapters.is_empty() {
        panic!("expected book with at least one chapter!");
    }
    let parsed = segment_book(&book, segmentation_mode);

    let mut word_frequencies: HashMap<&str, u64> = HashMap::new();
    let mut word_occurrences: HashMap<&str, &Chapter> = HashMap::new();
    for (i, chapter) in book.chapters.iter().enumerate() {
        if i == 0 {
            update_word_info(
                parsed.title_cut.iter().map(|w| w.as_str()).collect(),
                chapter,
                &mut word_frequencies,
                &mut word_occurrences,
            );
        }
        update_word_info(
            parsed
                .chapter_cuts
                .get(i)
                .unwrap()
                .cut
                .iter()
                .map(|w| w.as_str())
                .collect(),
            chapter,
            &mut word_frequencies,
            &mut word_occurrences,
        );
    }
    word_occurrences
        .into_iter()
        .map(|(word, chapter)| {
            (
                word.to_string(),
                (chapter, *word_frequencies.get(word).unwrap()),
            )
        })
        .collect()
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
