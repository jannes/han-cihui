use crate::errors::AppError;
use crate::errors::AppError::EpubParseError;
use epub::doc::{EpubDoc, NavPoint};
use scraper::Html;
use std::path::PathBuf;

pub struct Chapter {
    pub(crate) title: String,
    pub(crate) content: String,
    pub(crate) index: u32,
}

impl Chapter {
    pub fn get_numbered_title(&self) -> String {
        format!("{:04}-{}", self.index, self.title)
    }
    pub fn as_json(&self) -> String {
        unimplemented!()
    }
}

pub struct Book {
    pub(crate) title: String,
    pub(crate) author: String,
    pub(crate) chapters: Vec<Chapter>,
}

impl Book {
    pub fn as_json(&self) -> String {
        unimplemented!()
    }
}

pub fn open_as_book(filename: &str) -> Result<Book, AppError> {
    let edoc = EpubDoc::new(filename)
        .map_err(|_e| EpubParseError(format!("failed to create EpubDoc for {}", filename)))?;
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
    if matches.len() > 1 {
        panic!(
            "error when converting epubdoc to book: found multiple chapter matches for resource"
        );
    }
    match matches.get(0) {
        Some(navp) => Some(NavPoint {
            label: navp.label.to_owned(),
            content: navp.content.to_owned(),
            play_order: navp.play_order,
        }),
        None => None,
    }
}

fn html_to_text(html: &str) -> String {
    let fragment = Html::parse_fragment(html);
    let mut result = String::new();
    for node in fragment.tree {
        match node {
            scraper::Node::Text(text) => result.push_str(text.text.as_ref()),
            _ => {}
        }
    }
    result
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(|line| line.to_owned() + "\n")
        .collect()
}

fn get_book_from_edoc(mut edoc: EpubDoc) -> Result<Book, AppError> {
    let title = edoc
        .mdata("title")
        .expect("malformatted epub, did not contain title metadata");
    let author = edoc.mdata("creator");

    let mut chapters: Vec<Chapter> = Vec::new();
    let mut current_resource = edoc.get_current_id();
    let mut current_chapter = NavPoint {
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
        if chapter_match.is_some() {
            chapters.push(Chapter {
                title: current_chapter.label,
                content: html_to_text(&current_chapter_content),
                index: index,
            });
            current_chapter = chapter_match.unwrap();
            current_chapter_content = String::new();
        }
        current_chapter_content.push_str(current_resource_content.as_str());

        index += 1;
        if edoc.go_next().is_err() {
            break;
        }
        current_resource = edoc.get_current_id();
    }
    chapters.push(Chapter {
        title: current_chapter.label,
        content: html_to_text(&current_chapter_content),
        index: index,
    });

    Ok(Book {
        title: title,
        author: author.unwrap_or("unknown".to_string()),
        chapters: chapters,
    })
}

#[cfg(test)]
mod tests {
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
}
