use anyhow::Result;
use epubparse::epub_to_book;
use epubparse::types::Book;
use epubparse::types::Chapter;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub struct FlatBook {
    pub title: String,
    pub author: String,
    pub preface_content: String,
    pub chapters: Vec<FlatChapter>,
}

impl FlatBook {
    pub fn as_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

#[derive(Serialize, Deserialize)]
pub struct FlatChapter {
    pub title: String,
    pub content: String,
    pub index: usize,
}

impl FlatChapter {
    pub fn get_numbered_title(&self) -> String {
        format!("{:04}-{}", self.index, self.title)
    }
}

pub fn open_as_flat_book(filename: &str) -> Result<FlatBook> {
    let path_no_escaped_whitespace: String = filename.split('\\').into_iter().collect();
    let path = PathBuf::from(path_no_escaped_whitespace);
    let book = open_epub_as_book(&path)?;
    // flatten book such that there will be at least 4 chapters
    // try with iteratively higher depths
    let mut depth = 1;
    let mut flat_book = flatten_book(&book, depth);
    while depth <= 3 {
        if flat_book.chapters.len() > 5 {
            return Ok(flat_book);
        }
        depth += 1;
        flat_book = flatten_book(&book, depth);
    }
    Ok(flat_book)
}

fn open_epub_as_book(filepath: &Path) -> Result<Book> {
    let bytes = fs::read(filepath)?;
    Ok(epub_to_book(&bytes)?)
}

fn flatten_chapter(chapter: &Chapter) -> Chapter {
    let title = chapter.title.to_string();
    let mut text = vec![chapter.text.to_string()];
    for subchapter in &chapter.subchapters {
        let flattened = flatten_chapter(subchapter);
        text.push(flattened.title);
        text.push(flattened.text);
    }
    Chapter {
        title,
        text: text.join("\n"),
        subchapters: vec![],
    }
}

fn flatten_book(book: &Book, depth: u32) -> FlatBook {
    fn flatten_subchapters(chapter: &Chapter, depth: u32) -> Vec<Chapter> {
        if depth == 1 {
            vec![flatten_chapter(chapter)]
        } else {
            let flattened = Chapter {
                title: chapter.title.to_string(),
                text: chapter.text.to_string(),
                subchapters: vec![],
            };
            let flat_subchapters: Vec<Chapter> = chapter
                .subchapters
                .iter()
                .flat_map(|ch| flatten_subchapters(ch, depth - 1))
                .collect();
            let mut flattened = vec![flattened];
            flattened.extend(flat_subchapters);
            flattened
        }
    }

    let flat_chapters: Vec<FlatChapter> = book
        .chapters
        .iter()
        .flat_map(|chapter| flatten_subchapters(chapter, depth))
        .enumerate()
        .map(|(i, chapter)| FlatChapter {
            title: chapter.title,
            content: chapter.text,
            index: i,
        })
        .collect();

    FlatBook {
        title: book.title.clone(),
        author: book.author.clone().unwrap_or_else(|| "".to_string()),
        preface_content: book.preface_content.clone(),
        chapters: flat_chapters,
    }
}

#[cfg(test)]
mod tests {
    use crate::ebook::*;
    use epubparse::types::Book;
    use epubparse::types::Chapter;

    fn get_example_book() -> Book {
        Book {
            title: "example title".to_string(),
            author: Some("example author".to_string()),
            preface_content: "example preface".to_string(),
            chapters: vec![
                Chapter {
                    title: "Ch1".to_string(),
                    text: "1 text".to_string(),
                    subchapters: vec![Chapter {
                        title: "Ch1.1".to_string(),
                        text: "1.1 text".to_string(),
                        subchapters: vec![Chapter {
                            title: "Ch1.1.1".to_string(),
                            text: "1.1.1 text".to_string(),
                            subchapters: vec![],
                        }],
                    }],
                },
                Chapter {
                    title: "Ch2".to_string(),
                    text: "2 text".to_string(),
                    subchapters: vec![],
                },
            ],
        }
    }

    #[test]
    fn flatten_book_depth_1() {
        let book = get_example_book();
        let flattened = flatten_book(&book, 1);
        assert_eq!(flattened.chapters.len(), 2);
        let chapter1 = flattened.chapters.get(0).unwrap();
        let chapter2 = flattened.chapters.get(1).unwrap();
        assert!(chapter1.content.contains("1.1 text"));
        assert!(chapter1.content.ends_with("1.1.1 text"));
        assert!(chapter2.content.ends_with("2 text"));
    }

    #[test]
    fn flatten_book_depth_2() {
        let book = get_example_book();
        let flattened = flatten_book(&book, 2);
        assert_eq!(flattened.chapters.len(), 3);
        let chapter1 = flattened.chapters.get(0).unwrap();
        let chapter2 = flattened.chapters.get(1).unwrap();
        assert!(chapter1.content.ends_with("1 text"));
        assert!(chapter2.content.ends_with("1.1.1 text"));
    }

    #[test]
    fn book_to_json() {
        let chapter = FlatChapter {
            title: "一".to_string(),
            content: "这是第一章".to_string(),
            index: 1,
        };

        let book = FlatBook {
            title: "欢乐英雄".to_string(),
            author: "古龙".to_string(),
            preface_content: "".to_string(),
            chapters: vec![chapter],
        };

        assert_eq!("{\"title\":\"欢乐英雄\"", &book.as_json()[0..23]);
    }
}
