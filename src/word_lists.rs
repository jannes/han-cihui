use crate::config::{get_data_dir, tagging_socket_path, TAGGER_BIN};
use crate::extraction::ExtractionItem;
use crate::segmentation::BookSegmentation;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::fs;
use std::io::Write;
use std::os::unix::net::UnixListener;
use std::process::Command;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::analysis::AnalysisQuery;

#[derive(Clone)]
pub struct WordListMetadata {
    pub id: i64,
    pub book_name: String,
    pub author_name: String,
    pub create_time: SystemTime,
    pub analysis_query: AnalysisQuery,
}

impl Display for WordListMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.author_name.is_empty() {
            write!(f, "{}_{}", self.book_name, self.analysis_query)
        } else {
            write!(
                f,
                "{}-{}_{}",
                self.author_name, self.book_name, self.analysis_query
            )
        }
    }
}

pub struct WordList {
    pub metadata: WordListMetadata,
    pub words_per_chapter: Vec<ChapterWords>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChapterWords {
    pub chapter_name: String,
    pub tagged_words: Vec<TaggedWord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaggedWord {
    pub word: String,
    pub category: Option<Category>,
}

impl TaggedWord {
    pub fn new(word: &str) -> Self {
        Self {
            word: word.to_string(),
            category: None,
        }
    }

    pub fn tag(&mut self, category: Category) {
        self.category = Some(category);
    }

    pub fn reset(&mut self) {
        self.category = None;
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Category {
    Learn,
    NotLearn,
    Ignore,
}

// construct word list from book and analysis query/result
pub fn construct_word_list(
    book: &BookSegmentation,
    title: &str,
    author: &str,
    analysis_query: AnalysisQuery,
    unknown_words_to_save: &HashSet<&ExtractionItem>,
) -> WordList {
    let chapter_titles: Vec<&str> = book.chapter_cuts.iter().map(|c| c.title.as_str()).collect();
    let mut chapter_vocabulary: HashMap<&str, HashSet<&ExtractionItem>> = chapter_titles
        .iter()
        .map(|chapter_title| (*chapter_title, HashSet::new()))
        .collect();
    for item in unknown_words_to_save {
        chapter_vocabulary
            .get_mut(item.first_location.as_str())
            .unwrap()
            .insert(item);
    }
    let words_per_chapter: Vec<ChapterWords> = chapter_titles
        .iter()
        .map(|chapter_name| {
            let tagged_words: Vec<TaggedWord> = chapter_vocabulary
                .get(chapter_name)
                .unwrap()
                .iter()
                .map(|item| TaggedWord::new(item.word.as_str()))
                .collect();
            ChapterWords {
                chapter_name: chapter_name.to_string(),
                tagged_words,
            }
        })
        .collect();
    let metadata = WordListMetadata {
        id: -1,
        book_name: title.to_string(),
        author_name: author.to_string(),
        create_time: SystemTime::now(),
        analysis_query,
    };
    WordList {
        metadata,
        words_per_chapter,
    }
}

pub fn tag_words(words: &mut Vec<TaggedWord>) {
    let socket = tagging_socket_path();
    if socket.exists() {
        fs::remove_file(&socket).unwrap();
    }
    let listener = UnixListener::bind(socket).expect("could not bind to socket");

    let _c = Command::new(TAGGER_BIN)
        .env("DATA_DIR", get_data_dir())
        .spawn()
        .expect("could not spawn han-shaixuan");

    let mut stream = listener
        .incoming()
        .next()
        .expect("got none")
        .expect("could not get conn stream");

    let words_serialized: Vec<u8> = serde_json::to_string(&words)
        .expect("could not serialize words")
        .bytes()
        .collect();
    let n = words_serialized.len() as u32;
    let n = n.to_be_bytes();
    stream
        .write_all(&n[0..4])
        .expect("could not write n to stream");
    stream
        .write_all(&words_serialized)
        .expect("could not write words to stream");

    let tagged_words: Vec<TaggedWord> =
        serde_json::from_reader(stream).expect("could not read/deserialize from stream");
    *words = tagged_words;
}
