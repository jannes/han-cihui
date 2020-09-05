use epub::doc::{EpubDoc, NavPoint};
use jieba_rs::Jieba;
use scraper::{Html, Selector};

use crate::errors::AppError;
use crate::errors::AppError::EpubParseError;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::iter::Map;
use std::path::{Path, PathBuf};
use unicode_segmentation::{UnicodeSegmentation, UnicodeSentences};

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
pub struct Book {
    pub(crate) title: String,
    pub(crate) author: String,
    pub(crate) chapters: Vec<Chapter>,
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

pub fn open_as_book(filename: &str) -> Result<Book, AppError> {
    let edoc = EpubDoc::new(filename)
        .map_err(|e| EpubParseError(format!("failed to create EpubDoc for {}", filename)))?;
    get_book_from_edoc(edoc)
}

pub fn extract_vocabulary<'a>(book: &'a Book) -> ExtractionResult {
    if book.chapters.len() < 1 {
        panic!("expected book with at least one chapter!");
    }
    let jieba = Jieba::new();
    let mut word_frequencies: HashMap<&str, u64> = HashMap::new();
    let mut word_occurences: HashMap<&str, String> = HashMap::new();
    // closure captures mutable state variables, so also needs to be mutable
    // lifetime annotation for words vector needed, as word refs are stored in captured variables,
    // which outlive the closure's scope (but not the whole function's scope)
    let mut update_word_info = |words: Vec<&'a str>, current_chapter: &Chapter| -> () {
        for word in &words {
            let mut frequency = 1;
            if word_frequencies.contains_key(word) {
                frequency += word_frequencies.get(word).unwrap();
            } else {
                word_occurences.insert(word, current_chapter.get_numbered_title());
            }
            word_frequencies.insert(word, frequency);
        }
        ()
    };
    update_word_info(jieba.cut(&book.title, true), book.chapters.get(0).unwrap());
    update_word_info(jieba.cut(&book.author, true), book.chapters.get(0).unwrap());
    for chapter in &book.chapters {
        update_word_info(jieba.cut(&chapter.title, true), chapter);
        update_word_info(jieba.cut(&chapter.content, true), chapter);
    }

    let mut word_count: u64 = 0;
    let mut character_count: u64 = 0;
    let mut extraction_items: HashSet<ExtractionItem> = HashSet::new();
    let mut char_freq_map: HashMap<String, u64> = HashMap::new();
    let zh_word_occurrences: Vec<(&str, String)> = word_occurences
        .into_iter()
        .filter(|(w, l)| contains_hanzi(w))
        .collect();
    for (word, location) in zh_word_occurrences {
        let frequency = *word_frequencies.get(word).unwrap();
        word_count += frequency;
        let characters = word_to_hanzi(word);
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
            word: word.to_string(),
            frequency,
            location,
        };
        extraction_items.insert(extraction_item);
    }

    return ExtractionResult {
        word_count,
        character_count,
        vocabulary_info: extraction_items,
        char_freq_map,
    };
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

fn extract_from_string(s: &str) -> Vec<&str> {
    Jieba::new().cut(s, false)
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
    use crate::extraction::{contains_hanzi, extract_from_string, open_as_book};

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
}
