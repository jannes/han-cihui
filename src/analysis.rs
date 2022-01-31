use crate::{
    extraction::{word_to_hanzi, ExtractionItem, ExtractionResult},
    vocabulary::get_known_chars,
};
use std::collections::{HashMap, HashSet};

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub struct AnalysisQuery {
    pub min_occurrence_words: u64,
    pub min_occurrence_unknown_chars: Option<u64>,
}

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub struct AnalysisInfo {
    pub total_words: u64,
    pub total_chars: u64,
    pub unique_words: u64,
    pub unique_chars: u64,
    pub unknown_total_words: u64,
    pub unknown_total_chars: u64,
    pub unknown_unique_words: u64,
    pub unknown_unique_chars: u64,
}

/// Get all items that fulfill the min occurrence conditions
#[allow(clippy::unnecessary_unwrap)]
pub fn get_filtered_extraction_items<'a>(
    extraction_res: &'a ExtractionResult,
    min_occurrence_words: u64,
    known_words: &HashSet<String>,
    min_occurrence_unknown_chars: Option<u64>,
) -> HashSet<&'a ExtractionItem> {
    let known_chars = get_known_chars(known_words);
    let all_char_frequencies = ext_item_set_to_char_freq(&extraction_res.iter().collect());
    let unknown_char_frequencies: HashMap<&str, u64> = all_char_frequencies
        .iter()
        .filter(|(c, _freq)| !known_chars.contains(*c))
        .map(|(c, freq)| (c.as_str(), *freq))
        .collect();
    // closure that determines if a single item fulfills occurrence condition
    let occurrence_condition = |extraction_item: &ExtractionItem| {
        let min_occurring_words = extraction_item.frequency >= min_occurrence_words;
        if !min_occurring_words && min_occurrence_unknown_chars.is_some() {
            // if one character in word is both unknown and occurs at least min_occurrence_unknown_chars in total
            word_to_hanzi(&extraction_item.word)
                .iter()
                .filter_map(|hanzi| unknown_char_frequencies.get(hanzi))
                .any(|freq| *freq >= min_occurrence_unknown_chars.unwrap())
        } else {
            min_occurring_words
        }
    };
    extraction_res
        .iter()
        .filter(|item| occurrence_condition(item))
        .collect()
}

/// Get analysis info about words/chars for raw extraction result
///
/// min_occurrence_words: the minimum frequency for a word to be included in analysis
/// min_occurrence_unknown_chars:
///     if Some(amount), also include all words that include a character
///     that overall occurrs at least this amount and is unknown
pub fn get_analysis_info(
    extraction_res: &ExtractionResult,
    min_occurrence_words: u64,
    known_words: &HashSet<String>,
    min_occurrence_unknown_chars: Option<u64>,
) -> AnalysisInfo {
    let known_chars = get_known_chars(known_words);
    let vocabulary_min_occurring = get_filtered_extraction_items(
        extraction_res,
        min_occurrence_words,
        known_words,
        min_occurrence_unknown_chars,
    );
    let total_words: u64 = vocabulary_min_occurring
        .iter()
        .map(|item| item.frequency)
        .sum();
    let char_freq_min_occur: HashMap<String, u64> =
        ext_item_set_to_char_freq(&vocabulary_min_occurring);
    let total_chars: u64 = char_freq_min_occur.iter().map(|(_char, freq)| freq).sum();
    let unique_words = vocabulary_min_occurring.len() as u64;
    let unique_chars = char_freq_min_occur.len() as u64;

    /* UNKNOWN MIN OCCURRING WORDS/CHARS */
    let unknown_voc_min_occ: HashSet<&ExtractionItem> = vocabulary_min_occurring
        .iter()
        .copied()
        .filter(|item| !known_words.contains(&item.word))
        .collect();
    let unknown_total_words: u64 = unknown_voc_min_occ.iter().map(|item| item.frequency).sum();
    let unknown_char_min_occur: HashMap<&String, u64> = char_freq_min_occur
        .iter()
        .filter(|(hanzi, _freq)| !known_chars.contains(hanzi.as_str()))
        .map(|(hanzi, freq)| (hanzi, *freq))
        .collect();
    let unknown_total_chars: u64 = unknown_char_min_occur
        .iter()
        .map(|(_char, freq)| freq)
        .sum();
    let unknown_unique_words = unknown_voc_min_occ.len() as u64;
    let unknown_unique_chars = unknown_char_min_occur.len() as u64;

    AnalysisInfo {
        total_words,
        total_chars,
        unique_words,
        unique_chars,
        unknown_total_words,
        unknown_total_chars,
        unknown_unique_words,
        unknown_unique_chars,
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
