use std::collections::HashMap;

pub use jieba_rs::Jieba;
use unicode_segmentation::UnicodeSegmentation;

pub const F2J_TEXT: &str = include_str!("../1to1_fan-jian.txt");
pub const J2F_TEXT: &str = include_str!("../1to1_jian-fan.txt");

pub fn map_text(input_text: &str, fan2jian: bool) -> String {
    let mapping = get_mapping(fan2jian);
    let jieba = Jieba::new();
    let segmented = jieba.cut(input_text, false);
    segmented
        .into_iter()
        .map(|word| map_word(word, &mapping))
        .collect()
}

fn map_word(word: &str, mapping: &HashMap<String, String>) -> String {
    match mapping.get(word) {
        // if whole word is in dict, return mapped entry
        Some(mapped) => mapped.to_string(),
        // if not, map each hanzi separately
        // if hanzi has no mapping, keep original
        None => {
            let hanzis = word_to_hanzi(word);
            hanzis
                .into_iter()
                .map(|hanzi| match mapping.get(hanzi) {
                    Some(mapped) => mapped.to_string(),
                    None => hanzi.to_string(),
                })
                .collect()
        }
    }
}

pub fn get_mapping(fan2jian: bool) -> HashMap<String, String> {
    let text = if fan2jian { F2J_TEXT } else { J2F_TEXT };
    text.lines()
        .map(|line| {
            let split: Vec<_> = line.split(',').collect();
            let key = split[0];
            let val = split[1].trim();
            (key.to_string(), val.to_string())
        })
        .collect()
}

pub fn word_to_hanzi(word: &str) -> Vec<&str> {
    UnicodeSegmentation::graphemes(word, true).collect::<Vec<&str>>()
}
