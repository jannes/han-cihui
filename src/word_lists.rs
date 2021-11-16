use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::analysis::AnalysisQuery;

pub struct WordListMetadata {
    pub id: i64,
    pub book_name: String,
    pub author_name: String,
    pub create_time: SystemTime,
    pub analysis_query: AnalysisQuery,
}

pub struct WordList {
    pub metadata: WordListMetadata,
    pub words_per_chapter: Vec<ChapterWords>,
}

#[derive(Serialize, Deserialize)]
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
