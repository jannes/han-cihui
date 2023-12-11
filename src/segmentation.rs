use crate::ebook::FlatBook;
use crate::extraction::{contains_hanzi, word_to_hanzi};
use crate::fan2jian::get_mapping;
use jieba_rs::Jieba;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str;

#[derive(Serialize, Deserialize, Clone)]
pub struct ChapterSegmentation {
    pub title: String,
    pub cut: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BookSegmentation {
    pub title: String,
    pub chapter_cuts: Vec<ChapterSegmentation>,
}

pub fn segment_text(
    text: &str,
    jieba: &Jieba,
    mapping_fan2jian: &HashMap<String, String>,
    mapping_jian2fan: &HashMap<String, String>,
) -> Vec<String> {
    let chunks = jieba.cut(text, false);
    let mut segmented: Vec<String> = Vec::with_capacity(chunks.len());
    for chunk in chunks {
        if !contains_hanzi(chunk) {
            continue;
        }
        if let Some(word) = mapping_fan2jian.get(chunk) {
            segmented.push(word.to_owned());
        } else if mapping_jian2fan.contains_key(chunk) {
            segmented.push(chunk.to_owned());
        } else {
            let hanzis = word_to_hanzi(chunk);
            for hanzi in hanzis {
                if let Some(hanzi) = mapping_fan2jian.get(hanzi) {
                    segmented.push(hanzi.to_owned());
                }
            }
        }
    }
    segmented
}

pub fn segment_book(book: &FlatBook) -> BookSegmentation {
    let jieba = Jieba::new();
    let fan2jian = get_mapping(true);
    let jian2fan = get_mapping(false);

    let segment = |text: &String| segment_text(text, &jieba, &fan2jian, &jian2fan);

    // preface cut include title and author
    let mut preface_cut = segment(&book.preface_content);
    preface_cut.extend(segment(&book.title));
    preface_cut.extend(segment(&book.author));

    // preface is first chapter
    let mut chapter_segmentations: Vec<ChapterSegmentation> =
        Vec::with_capacity(book.chapters.len() + 1);
    let preface_segmentation = ChapterSegmentation {
        title: "Preface".to_owned(),
        cut: preface_cut,
    };
    chapter_segmentations.push(preface_segmentation);

    for chapter in &book.chapters {
        let mut cut = segment(&chapter.content);
        cut.extend(segment(&chapter.title));
        let chapter_segmentation = ChapterSegmentation {
            title: chapter.title.clone(),
            cut,
        };
        chapter_segmentations.push(chapter_segmentation);
    }

    BookSegmentation {
        title: book.title.clone(),
        chapter_cuts: chapter_segmentations,
    }
}
