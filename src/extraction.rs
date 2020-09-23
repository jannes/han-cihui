use std::collections::{HashMap, HashSet};

use jieba_rs::Jieba;
use regex::Regex;
use tempfile::tempdir;
use unicode_segmentation::UnicodeSegmentation;

use crate::ebook::{Book, Chapter};
use crate::python_interop::run_python;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;

pub trait Extractor {
    fn extract<'a>(&self, book: &'a Book) -> HashMap<String, (&'a Chapter, u64)>;
}

pub struct Pkuseg {}

#[derive(Serialize, Deserialize)]
struct PkusegChapterSegmentation {
    title: String,
    cut: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct PkusegBookSegmentation {
    cut: Vec<String>,
    chapters: Vec<PkusegChapterSegmentation>,
}

fn update_word_info<'a, 'b>(
    words: Vec<&'a str>,
    current_chapter: &'b Chapter,
    word_frequencies: &mut HashMap<&'a str, u64>,
    word_occurrences: &mut HashMap<&'a str, &'b Chapter>,
) -> () {
    for word in words {
        let mut frequency = 1;
        if word_frequencies.contains_key(word) {
            frequency += word_frequencies.get(word).unwrap();
        } else {
            word_occurrences.insert(word, current_chapter);
        }
        word_frequencies.insert(word, frequency);
    }
    ()
}

fn pkuseg_tmp_file(book_json: &str) -> PkusegBookSegmentation {
    let dir = tempdir().expect("expect successful creation of tempdir");
    let file_path = dir.path().join("tmp-book.json");
    let mut file = File::create(&file_path).expect("expect successful creation of tempfile");
    file.write_all(book_json.as_bytes())
        .expect("expect successful write to tempfile");
    let with_open = format!(
        "    with open('{}') as f:",
        file_path
            .into_os_string()
            .into_string()
            .expect("expect successful conversion from tempfile path to string")
    );
    let func_call = format!("        segment_dump(f.read())");
    let python_program = include_str!("pkuseg_segment_book.py");
    let full_python_program = vec![python_program, &with_open, &func_call].join("\n");
    let output = run_python(&full_python_program);
    drop(file);
    dir.close()
        .expect("expect closing of tempdir to be successful");
    serde_json::from_str(output.as_str())
        .expect("expected valid json structure in format of PkusegBookSegmentation struct")
}

fn pkuseg_stdin(book_json: &str) -> PkusegBookSegmentation {
    let data_assign = format!("    data = \"\"\"{}\"\"\"", book_json);
    let func_call = format!("    segment_dump(data)");
    let python_program = include_str!("pkuseg_segment_book.py");
    let full_python_program = vec![python_program, &data_assign, &func_call].join("\n");
    let output = run_python(&full_python_program);
    serde_json::from_str(output.as_str())
        .expect("expected valid json structure in format of PkusegBookSegmentation struct")
}

impl Extractor for Pkuseg {
    fn extract<'a>(&self, book: &'a Book) -> HashMap<String, (&'a Chapter, u64)> {
        if book.chapters.len() < 1 {
            panic!("expected book with at least one chapter!");
        }
        let book_json = book.as_json();
        let parsed = pkuseg_tmp_file(&book_json);

        let mut word_frequencies: HashMap<&str, u64> = HashMap::new();
        let mut word_occurrences: HashMap<&str, &Chapter> = HashMap::new();
        for (i, chapter) in book.chapters.iter().enumerate() {
            if i == 0 {
                update_word_info(
                    parsed.cut.iter().map(|w| w.as_str()).collect(),
                    chapter,
                    &mut word_frequencies,
                    &mut word_occurrences,
                );
            }
            update_word_info(
                parsed
                    .chapters
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
}

impl Extractor for Jieba {
    fn extract<'a>(&self, book: &'a Book) -> HashMap<String, (&'a Chapter, u64)> {
        if book.chapters.len() < 1 {
            panic!("expected book with at least one chapter!");
        }
        let mut word_frequencies: HashMap<&str, u64> = HashMap::new();
        let mut word_occurences: HashMap<&str, &Chapter> = HashMap::new();
        // closure captures mutable state variables, so also needs to be mutable
        // lifetime annotation for words vector needed, as word refs are stored in captured variables,
        // which outlive the closure's scope (but not the whole function's scope)
        let mut update_word_info = |words: Vec<&'a str>, current_chapter: &'a Chapter| -> () {
            for word in &words {
                let mut frequency = 1;
                if word_frequencies.contains_key(word) {
                    frequency += word_frequencies.get(word).unwrap();
                } else {
                    word_occurences.insert(word, current_chapter);
                }
                word_frequencies.insert(word, frequency);
            }
            ()
        };
        update_word_info(self.cut(&book.title, true), book.chapters.get(0).unwrap());
        update_word_info(self.cut(&book.author, true), book.chapters.get(0).unwrap());
        for chapter in &book.chapters {
            update_word_info(self.cut(&chapter.title, true), chapter);
            update_word_info(self.cut(&chapter.content, true), chapter);
        }
        word_occurences
            .into_iter()
            .map(|(word, chapter)| {
                (
                    word.to_owned(),
                    (chapter, *word_frequencies.get(word).unwrap()),
                )
            })
            .collect()
    }
}

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

pub fn extract_vocab(book: &Book, extractor: &impl Extractor) -> ExtractionResult {
    let word_occur_freq = extractor.extract(book);
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

#[allow(dead_code)]
fn extract_from_string(s: &str) -> Vec<&str> {
    Jieba::new().cut(s, false)
}

#[cfg(test)]
mod tests {
    use crate::extraction::{contains_hanzi, extract_from_string, PkusegBookSegmentation};
    use crate::python_interop::run_python;

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

    #[test]
    fn use_pkuseg() {
        let book_json = include_str!("../test_resources/book.json");
        let data_assign = format!("    data = \"\"\"{}\"\"\"", book_json);
        let func_call = format!("    segment_dump(data)");
        let python_program = include_str!("pkuseg_segment_book.py");
        let full_python_program = vec![python_program, &data_assign, &func_call].join("\n");
        let output = run_python(&full_python_program);
        let parsed: PkusegBookSegmentation = serde_json::from_str(output.as_str())
            .expect("expected valid json structure in format of PkusegBookSegmentation struct");
        let expected_cut = vec!["欢乐".to_string(), "英雄".to_string(), "古龙".to_string()];
        assert_eq!(expected_cut, parsed.cut);
    }
}
