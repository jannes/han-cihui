use crate::dictionary::get_dictionary_words;
use crate::ext_item_set_to_char_freq;
use crate::extraction::{word_to_hanzi, ExtractionItem, ExtractionResult};
use prettytable::Table;
use std::collections::{HashMap, HashSet};

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
