use anyhow::{anyhow, Context, Result};
use epub::doc::{EpubDoc, NavPoint};
use scraper::Html;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub struct Chapter {
    pub(crate) title: String,
    pub(crate) content: String,
    pub(crate) index: u32,
}

impl Chapter {
    pub fn get_numbered_title(&self) -> String {
        format!("{:04}-{}", self.index, self.title)
    }
}

#[derive(Serialize, Deserialize)]
pub struct Book {
    pub(crate) title: String,
    pub(crate) author: String,
    pub(crate) chapters: Vec<Chapter>,
}

impl Book {
    pub fn as_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

pub fn open_as_book(filename: &str) -> Result<Book> {
    let edoc = EpubDoc::new(filename)
        .map_err(|_e| anyhow!("failed to create EpubDoc for {}", filename))?;

    get_book_from_edoc(edoc)
}

fn get_matching_navpoint(edoc: &EpubDoc, resource_path: &PathBuf) -> Option<NavPoint> {
    let matches: Vec<&NavPoint> = edoc
        .toc
        .iter()
        .filter(|navp| {
            navp.content
                .to_str()
                .unwrap()
                .contains(resource_path.to_str().unwrap())
        })
        .collect();
    // if there are are multiple matches they are most likely chapter and subchapter
    // choose the last match, which should be the subchapter,
    // as the chapter is just a container for the subchapters (usually)
    if let Some(navp) = matches.last() {
        Some(NavPoint {
            label: navp.label.to_owned(),
            content: navp.content.to_owned(),
            play_order: navp.play_order,
        })
    } else {
        None
    }
}

// TODO: make this work
fn clean_html_entities(text: &str) -> String {
    text.replace("\u{00A0}", "J")
}

fn html_to_text(html: &str) -> String {
    let fragment = Html::parse_fragment(html);
    let mut result = String::new();
    for node in fragment.tree {
        if let scraper::Node::Text(text) = node {
            result.push_str(text.text.as_ref())
        }
    }
    result
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string() + "\n")
        .collect()
}

fn get_book_from_edoc(mut edoc: EpubDoc) -> Result<Book> {
    let title = edoc
        .mdata("title")
        .context("malformatted epub, did not contain title metadata")?;
    let author = edoc.mdata("creator");
    let mut chapters: Vec<Chapter> = Vec::new();
    let mut current_resource = edoc.get_current_id();
    let current_chapter = NavPoint {
        label: "".to_string(),
        content: PathBuf::new(),
        play_order: 0,
    };
    let mut current_chapter_content = String::new();
    let mut index: u32 = 0;
    while current_resource.is_ok() {
        let current_resource_path: PathBuf = edoc
            .resources
            .get(current_resource.as_ref().unwrap())
            .unwrap()
            .0
            .clone();
        let current_resource_content = edoc
            .get_resource_str_by_path(&current_resource_path)
            .expect("invalid path to resource");

        // find chapter that matches current resource
        let chapter_match = get_matching_navpoint(&edoc, &current_resource_path);
        // if any chapter matches current resource update current chapter,
        // else current resource is still in old chapter
        if let Some(current_chapter) = chapter_match {
            chapters.push(Chapter {
                title: current_chapter.label,
                content: html_to_text(&current_chapter_content),
                index,
            });
            current_chapter_content = String::new();
            index += 1;
        }
        current_chapter_content.push_str(current_resource_content.as_str());

        if edoc.go_next().is_err() {
            break;
        }
        current_resource = edoc.get_current_id();
    }
    chapters.push(Chapter {
        title: clean_html_entities(&current_chapter.label),
        content: html_to_text(&current_chapter_content),
        index,
    });

    Ok(Book {
        title: clean_html_entities(&title),
        author: author.unwrap_or_else(|| "unknown".to_string()),
        chapters,
    })
}

#[cfg(test)]
mod tests {
    use crate::ebook::{open_as_book, Book, Chapter};

    #[test]
    fn parse_epub() {
        let book = open_as_book("test_resources/xusanguan.epub").unwrap();
        assert_eq!(book.author, "余华");
        assert_eq!(book.title, "许三观卖血记 (余华作品)");
        assert_eq!(book.chapters.len(), 36);
        assert_eq!(book.chapters.get(6).unwrap().title, "第一章");
        assert!(book
            .chapters
            .get(6)
            .unwrap()
            .content
            .contains("许三观是城里丝厂的送茧工，这一天他回到村里来看望他的爷爷。"));
        assert_eq!(book.chapters.get(34).unwrap().title, "第二十九章");
    }

    #[test]
    fn parse_epub2() {
        let book = open_as_book("test_resources/wangxiaobo-essays.epub").unwrap();
        assert_eq!(book.author, "王小波");
        assert_eq!(book.title, "王小波杂文集");
        assert_eq!(book.chapters.len(), 38);
        assert_eq!(book.chapters.get(4).unwrap().title, "花刺子模信使问题");
    }

    #[test]
    fn book_to_json() {
        let chapter = Chapter {
            title: "一".to_string(),
            content: "这是第一章".to_string(),
            index: 1,
        };

        let book = Book {
            title: "欢乐英雄".to_string(),
            author: "古龙".to_string(),
            chapters: vec![chapter],
        };

        assert_eq!("{\"title\":\"欢乐英雄\"", &book.as_json()[0..23]);
    }
}
