use crate::ebook::FlatBook;
use crate::extraction::{contains_hanzi, word_to_hanzi};
use crate::fan2jian::{self};
use jieba_rs::Jieba;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;
use std::process::{Command, Stdio};
use std::str;
use tempfile::tempdir;

#[derive(Serialize, Deserialize, Clone)]
pub struct ChapterSegmentation {
    pub title: String,
    pub cut: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BookSegmentation {
    pub title_cut: Vec<String>,
    pub chapter_cuts: Vec<ChapterSegmentation>,
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub enum SegmentationMode {
    Default,
    DictionaryOnly,
}

pub fn segment_book(book: &FlatBook, segmentation_mode: SegmentationMode) -> BookSegmentation {
    let dir = tempdir().expect("expect successful creation of tempdir");
    let file_path = dir.path().join("tmp-book.json");
    let mut file = File::create(&file_path).expect("expect successful creation of tempfile");
    let book_json = book.as_json();
    let book_json_simplified = fan2jian::map_text(&book_json, true);
    file.write_all(book_json_simplified.as_bytes())
        .expect("expect successful write to tempfile");
    let json_filepath = file_path
        .into_os_string()
        .into_string()
        .expect("expect successful conversion from tempfile path to string");

    let args = match segmentation_mode {
        SegmentationMode::Default => vec!["/usr/local/bin/han-segmenter", "-j", &json_filepath],
        SegmentationMode::DictionaryOnly => {
            vec!["/usr/local/bin/han-segmenter", "-j", &json_filepath, "-d"]
        }
    };

    let han_segmenter_process = match Command::new("bash")
        .args(&args)
        .stdout(Stdio::piped())
        .spawn()
    {
        Ok(process) => process,
        Err(err) => panic!(
            "could not spawn han-segmenter, make sure bash script is at /usr/local/bin/han-segmenter, error: {}",
            err
        ),
    };

    let output = match han_segmenter_process.wait_with_output() {
        Ok(output) => output,
        Err(why) => panic!(
            "failed to read stdout of han-segmenter process, error: {}",
            why
        ),
    };

    let output = if output.status.success() {
        str::from_utf8(&output.stdout)
            .expect("expected han-segmenter output to be utf8")
            .to_string()
    } else {
        panic!("han-segmenter return status non zero")
    };

    drop(file);
    dir.close()
        .expect("expect closing of tempdir to be successful");

    serde_json::from_str(output.as_str())
        .expect("expected valid json structure in format of BookSegmentation struct")
}

/// Segments text into list of words
/// converts from traditional to simplified
// TODO: improve this terrible implementation
pub fn extract_words(
    text: &str,
    jieba: &Jieba,
    mapping_fan2jian: &HashMap<String, String>,
    mapping_jian2fan: &HashMap<String, String>,
) -> HashSet<String> {
    let segmented = jieba.cut(text, false);
    // save references first to avoid repetive allocation of duplicates
    let mut words: HashSet<&str> = HashSet::with_capacity(10_000);
    for chunk in segmented {
        if !contains_hanzi(chunk) {
            continue;
        }
        if let Some(word) = mapping_fan2jian.get(chunk) {
            words.insert(word);
        } else if mapping_jian2fan.contains_key(chunk) {
            words.insert(chunk);
        } else {
            let hanzis = word_to_hanzi(chunk);
            for hanzi in hanzis {
                if let Some(hanzi) = mapping_fan2jian.get(hanzi) {
                    words.insert(hanzi);
                }
            }
        }
    }
    words.into_iter().map(|w| w.to_owned()).collect()
}
