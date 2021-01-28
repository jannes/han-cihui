use std::collections::HashSet;

pub fn get_dictionary_words() -> HashSet<String> {
    let words_str = include_str!("../dictionary_words.txt");
    words_str
        .split('\n')
        .map(|line| line.trim().to_string())
        .collect()
}

