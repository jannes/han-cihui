use crate::ebook::Book;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::process::{Command, Stdio};
use std::str;
use tempfile::tempdir;

#[derive(Serialize, Deserialize)]
pub struct ChapterSegmentation {
    pub title: String,
    pub cut: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct BookSegmentation {
    pub title_cut: Vec<String>,
    pub chapter_cuts: Vec<ChapterSegmentation>,
}

#[derive(Clone, Copy)]
pub enum SegmentationMode {
    Default,
    DictionaryOnly,
}

pub fn segment_book(book: &Book, segmentation_mode: SegmentationMode) -> BookSegmentation {
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

    // let stdin = han_segmenter_process.stdin.as_mut().unwrap();
    // let stdout = han_segmenter_process.stdout.as_mut().unwrap();
    // stdout.
    //
    // let title_cut = get_segments(&book.title, &mut han_segmenter_process, stdin);
    // let mut chapter_cuts = Vec::new();
    // for chapter in &book.chapters {
    //     let title_cut = get_segments(&chapter.title, &mut han_segmenter_process, stdin);
    //     let content_cut = get_segments(&chapter.content, &mut han_segmenter_process, stdin);
    //     chapter_cuts.push(ChapterSegmentation {
    //         title_cut,
    //         content_cut,
    //     })
    // }
    // BookSegmentation {
    //     title_cut,
    //     chapter_cuts,
    // }
}
//
// fn get_segments(
//     s: &str,
//     segmenter_process: &mut Child,
//     segmenter_stdin: &mut ChildStdin,
// ) -> Vec<String> {
//     if let Err(why) = segmenter_stdin.write(s.as_bytes()) {
//         panic!(
//             "failed to write to stdin of han-segmenter process, error: {}",
//             why
//         )
//     }
//     let output = match segmenter_process.wait_with_output() {
//         Ok(output) => output,
//         Err(why) => panic!(
//             "failed to read stdout of han-segmenter process, error: {}",
//             why
//         ),
//     };
//
//     if output.status.success() {
//         str::from_utf8(&output.stdout)
//             .expect("expected python output to be utf8")
//             .split('\n')
//             .map(|s| s.to_string())
//             .collect()
//     } else {
//         panic!("python return status non zero")
//     }
// }
