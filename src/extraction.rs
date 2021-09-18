use std::collections::{HashMap, HashSet};

use crate::ebook::{FlatBook, FlatChapter};
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
    pub(crate) vocabulary_info: HashSet<ExtractionItem>,
}

pub fn extract_vocab(book: &FlatBook, segmentation_mode: SegmentationMode) -> ExtractionResult {
    let word_occur_freq = extract(book, segmentation_mode);
    let mut vocabulary_info: HashSet<ExtractionItem> = HashSet::new();
    let mut char_freq_map: HashMap<String, u64> = HashMap::new();

    for (word, (chapter, frequency)) in word_occur_freq {
        if contains_hanzi(&word) {
            let characters = word_to_hanzi(&word);
            for character in characters {
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

    ExtractionResult { vocabulary_info }
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
    current_chapter: &'b FlatChapter,
    word_frequencies: &mut HashMap<&'a str, u64>,
    word_occurrences: &mut HashMap<&'a str, &'b FlatChapter>,
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

fn extract(
    book: &FlatBook,
    segmentation_mode: SegmentationMode,
) -> HashMap<String, (&FlatChapter, u64)> {
    if book.chapters.is_empty() {
        panic!("expected book with at least one chapter!");
    }
    let parsed = segment_book(book, segmentation_mode);

    let mut word_frequencies: HashMap<&str, u64> = HashMap::new();
    let mut word_occurrences: HashMap<&str, &FlatChapter> = HashMap::new();
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
