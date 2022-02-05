use crate::ebook::FlatBook;
use serde::{Deserialize, Serialize};
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
    file.write_all(book_json.as_bytes())
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
