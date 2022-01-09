use crate::ebook::FlatBook;
use crate::extraction::ExtractionItem;
use std::collections::{HashMap, HashSet};
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

// construct word list from book and analysis query/result
pub fn construct_word_list(
    book: &FlatBook,
    analysis_query: AnalysisQuery,
    unknown_words_to_save: &HashSet<&ExtractionItem>,
) -> WordList {
    let chapter_titles: Vec<String> = book
        .chapters
        .iter()
        .map(|chapter| chapter.get_numbered_title())
        .collect();
    let mut chapter_vocabulary: HashMap<&str, HashSet<&ExtractionItem>> = chapter_titles
        .iter()
        .map(|chapter_title| (chapter_title.as_str(), HashSet::new()))
        .collect();
    for item in unknown_words_to_save {
        chapter_vocabulary
            .get_mut(item.location.as_str())
            .unwrap()
            .insert(item);
    }
    let words_per_chapter: Vec<ChapterWords> = chapter_titles
        .iter()
        .map(|chapter_name| {
            let tagged_words: Vec<TaggedWord> = chapter_vocabulary
                .get(chapter_name.as_str())
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
        book_name: book.title.clone(),
        author_name: book.author.clone(),
        create_time: SystemTime::now(),
        analysis_query,
    };
    WordList {
        metadata,
        words_per_chapter,
    }
}
