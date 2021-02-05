use crate::extraction::{word_to_hanzi, ExtractionItem, ExtractionResult};
use crate::{ebook::Book, ext_item_set_to_char_freq};
use anyhow::{Context, Result};
use prettytable::Table;
use serde_json::{json, to_writer_pretty, Value};
use std::{
    collections::{HashMap, HashSet},
    fs::File,
};

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

pub fn get_filtered_extraction_items<'a>(
    extraction_res: &'a ExtractionResult,
    min_occurrence_words: u64,
    known_words: &HashSet<String>,
    min_occurrence_unknown_chars: Option<u64>,
) -> HashSet<&'a ExtractionItem> {
    let known_chars: HashSet<String> = known_words
        .iter()
        .flat_map(|w| word_to_hanzi(w))
        .map(|hanzi| hanzi.to_string())
        .collect();
    let all_char_frequencies =
        ext_item_set_to_char_freq(&extraction_res.vocabulary_info.iter().collect());
    let unknown_char_frequencies: HashMap<&str, u64> = all_char_frequencies
        .iter()
        .filter(|(c, _freq)| known_chars.contains(*c))
        .map(|(c, freq)| (c.as_str(), *freq))
        .collect();
    let occurrence_condition = |extraction_item: &ExtractionItem| {
        let min_occurring_words = extraction_item.frequency >= min_occurrence_words;
        if !min_occurring_words && min_occurrence_unknown_chars.is_some() {
            // if one character in word is both unknown and occurs at least min_occurrence_unknown_chars in total
            word_to_hanzi(&extraction_item.word)
                .iter()
                .map(|hanzi| unknown_char_frequencies.get(hanzi))
                .filter(|freq| freq.is_some())
                .any(|freq| *freq.unwrap() >= min_occurrence_unknown_chars.unwrap())
        } else {
            min_occurring_words
        }
    };
    extraction_res
        .vocabulary_info
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
    let known_chars: HashSet<String> = known_words
        .iter()
        .flat_map(|w| word_to_hanzi(w))
        .map(|hanzi| hanzi.to_string())
        .collect();
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

pub fn do_extraction_analysis(
    extraction_res: &ExtractionResult,
    min_occ: u64,
    known_words: HashSet<String>,
) -> HashSet<&ExtractionItem> {
    let known_chars: HashSet<&str> = known_words
        .iter()
        .flat_map(|word| word_to_hanzi(&word))
        .collect();
    /* ALL WORDS */
    let amount_unique_words = extraction_res.vocabulary_info.len();
    let amount_unique_chars = extraction_res.char_freq_map.len();
    let unknown_voc: HashSet<&ExtractionItem> = extraction_res
        .vocabulary_info
        .iter()
        .filter(|item| !known_words.contains(&item.word))
        .collect();
    let unknown_char: HashMap<&str, u64> = extraction_res
        .char_freq_map
        .iter()
        .map(|(k, v)| (k.as_str(), *v))
        .filter(|(k, _v)| !known_chars.contains(k))
        .collect();
    let amount_unknown_words: u64 = unknown_voc.iter().map(|item| item.frequency).sum();
    let amount_unknown_chars: u64 = unknown_char.iter().map(|(_k, v)| v).sum();

    /* MIN OCCURRING WORDS */
    let vocabulary_min_occurring: HashSet<&ExtractionItem> = extraction_res
        .vocabulary_info
        .iter()
        .filter(|item| item.frequency >= min_occ)
        .collect();
    let total_min_occurring_words: u64 = vocabulary_min_occurring
        .iter()
        .map(|item| item.frequency)
        .sum();

    let char_freq_min_occur: HashMap<String, u64> =
        ext_item_set_to_char_freq(&vocabulary_min_occurring);
    let total_char_min_occur: u64 = char_freq_min_occur.iter().map(|(_char, freq)| freq).sum();

    let unknown_voc_min_occ: HashSet<&ExtractionItem> = vocabulary_min_occurring
        .iter()
        .copied()
        .filter(|item| !known_words.contains(&item.word))
        .collect();
    let total_unknown_min_occur_words: u64 =
        unknown_voc_min_occ.iter().map(|item| item.frequency).sum();

    let unknown_char_min_occur: HashMap<&String, u64> = char_freq_min_occur
        .iter()
        .filter(|(hanzi, _freq)| !known_chars.contains(hanzi.as_str()))
        .map(|(hanzi, freq)| (hanzi, *freq))
        .collect();

    let total_unknown_char_min_occur: u64 = unknown_char_min_occur
        .iter()
        .map(|(_char, freq)| freq)
        .sum();

    let dict_words = get_dictionary_words();
    let dict_entries: HashSet<&ExtractionItem> = extraction_res
        .vocabulary_info
        .iter()
        .filter(|item| dict_words.contains(&item.word))
        .collect();
    let unknown_dict_entries: HashSet<&ExtractionItem> = dict_entries
        .iter()
        .filter(|item| !known_words.contains(&item.word))
        .copied()
        .collect();
    let total_amount_dict_entries: u64 = dict_entries.iter().map(|item| item.frequency).sum();
    let total_amount_unknown_dict_entries: u64 =
        unknown_dict_entries.iter().map(|item| item.frequency).sum();

    let minocc_dict_entries: HashSet<&ExtractionItem> = dict_entries
        .iter()
        .filter(|item| item.frequency >= min_occ)
        .copied()
        .collect();
    let minocc_unknown_dict_entries: HashSet<&ExtractionItem> = unknown_dict_entries
        .iter()
        .filter(|item| item.frequency >= min_occ)
        .copied()
        .collect();
    let total_amount_minocc_dict_entries: u64 =
        minocc_dict_entries.iter().map(|item| item.frequency).sum();
    let total_amount_minocc_unknown_dict_entries: u64 = minocc_unknown_dict_entries
        .iter()
        .map(|item| item.frequency)
        .sum();

    let mut table = Table::new();
    table.add_row(row![
        "",
        "all (dict_entries)",
        format!("min {} (dict_entries)", min_occ)
    ]);
    table.add_row(row![
        "total amount words",
        format!(
            "{} ({})",
            extraction_res.word_count, total_amount_dict_entries
        ),
        format!(
            "{} ({})",
            total_min_occurring_words, total_amount_minocc_dict_entries
        )
    ]);
    table.add_row(row![
        "total amount unknown words",
        format!(
            "{} ({})",
            amount_unknown_words, total_amount_unknown_dict_entries
        ),
        format!(
            "{} ({})",
            total_unknown_min_occur_words, total_amount_minocc_unknown_dict_entries
        )
    ]);
    table.add_row(row![
        "total amount characters",
        extraction_res.character_count,
        total_char_min_occur
    ]);
    table.add_row(row![
        "total amount unknown characters",
        amount_unknown_chars,
        total_unknown_char_min_occur
    ]);
    table.add_row(row![
        "amount unique words",
        format!("{} ({})", amount_unique_words, dict_entries.len()),
        format!(
            "{} ({})",
            vocabulary_min_occurring.len(),
            minocc_dict_entries.len()
        )
    ]);
    table.add_row(row![
        "amount unknown unique words",
        format!("{} ({})", unknown_voc.len(), unknown_dict_entries.len()),
        format!(
            "{} ({})",
            unknown_voc_min_occ.len(),
            minocc_unknown_dict_entries.len()
        )
    ]);
    table.add_row(row![
        "amount unique characters",
        amount_unique_chars,
        char_freq_min_occur.len()
    ]);
    table.add_row(row![
        "amount unknown unique characters",
        unknown_char.len(),
        unknown_char_min_occur.len()
    ]);
    table.printstd();
    unknown_voc_min_occ
}

pub fn save_filtered_extraction_info(
    book: &Book,
    filtered_extraction_set: &HashSet<&ExtractionItem>,
    outpath: &str,
) -> Result<()> {
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
    to_writer_pretty(&File::create(outpath)?, &output_json).context("failed to write result json")
}

pub fn get_dictionary_words() -> HashSet<String> {
    let words_str = include_str!("../dictionary.txt");
    words_str
        .split('\n')
        .map(|line| line.trim().to_string())
        .collect()
}
