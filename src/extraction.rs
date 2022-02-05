use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};

use crate::ebook::FlatBook;
use crate::segmentation::{segment_book, BookSegmentation, SegmentationMode};
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

pub fn extract_vocab(book: &FlatBook, segmentation_mode: SegmentationMode) -> ExtractionResult {
    if book.chapters.is_empty() {
        panic!("expected book with at least one chapter!");
    }
    let segmented = segment_book(book, segmentation_mode);
    extract_vocab_from_segmented(segmented)
}

pub fn extract_vocab_from_segmented(segmented_book: BookSegmentation) -> ExtractionResult {
    let mut word_frequencies: HashMap<&str, u64> = HashMap::new();
    let mut word_occurrences: HashMap<&str, &str> = HashMap::new();
    for (i, chapter) in segmented_book
        .chapter_cuts
        .iter()
        .map(|c| c.title.as_str())
        .enumerate()
    {
        if i == 0 {
            // include title in first chapter
            update_word_info(
                segmented_book.title_cut.iter(),
                chapter,
                &mut word_frequencies,
                &mut word_occurrences,
            );
        }
        update_word_info(
            segmented_book.chapter_cuts.get(i).unwrap().cut.iter(),
            chapter,
            &mut word_frequencies,
            &mut word_occurrences,
        );
    }
    let vocabulary = word_occurrences
        .into_iter()
        .filter(|(word, _)| contains_hanzi(word))
        .map(|(word, chapter)| ExtractionItem {
            word: word.to_string(),
            frequency: *word_frequencies.get(word).unwrap(),
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

fn update_word_info<'a, 'b>(
    words: impl Iterator<Item = &'a String>,
    current_chapter: &'b str,
    word_frequencies: &mut HashMap<&'a str, u64>,
    word_occurrences: &mut HashMap<&'a str, &'b str>,
) {
    for word in words {
        match word_frequencies.entry(word) {
            Entry::Occupied(o) => {
                *o.into_mut() += 1;
            }
            Entry::Vacant(v) => {
                v.insert(1);
                word_occurrences.insert(word, current_chapter);
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
