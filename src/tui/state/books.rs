use anyhow::Result;
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use rusqlite::Connection;

use crate::{
    extraction::word_to_hanzi,
    persistence::{db_books_select_all, db_words_select_known},
    segmentation::BookSegmentation,
};

pub enum BooksState {
    Uninitialized,
    Calculate(CalculatingState),
    Display(DisplayState),
    // String arg: partial file path
    Importing(String),
}

impl BooksState {
    pub fn init(db_connection: Arc<Mutex<Connection>>) -> Result<Self> {
        let books = db_books_select_all(&db_connection.lock().unwrap())?;
        let known_words = db_words_select_known(&db_connection.lock().unwrap())?;
        Ok(Self::Calculate(CalculatingState::new(books, known_words)))
    }
}

pub struct CalculatingState {
    books: Vec<BookSegmentation>,
    known_words: HashSet<String>,
}

impl CalculatingState {
    pub fn new(books: Vec<BookSegmentation>, known_words: HashSet<String>) -> Self {
        Self { books, known_words }
    }

    pub fn update(&self) -> BooksState {
        let mut books_with_stats = Vec::with_capacity(self.books.len());
        for book in &self.books {
            books_with_stats.push(get_enrich_book_with_stats(book.clone(), &self.known_words))
        }
        BooksState::Display(DisplayState::new(books_with_stats))
    }
}

pub fn get_enrich_book_with_stats(
    book: BookSegmentation,
    known_words: &HashSet<String>,
) -> BookWithStats {
    let mut word_sequence = Vec::new();
    word_sequence.extend(&book.title_cut);
    for chapter in &book.chapter_cuts {
        word_sequence.push(&chapter.title);
        word_sequence.extend(&chapter.cut);
    }

    let total_words = word_sequence.len();
    let mut total_chars = 0;
    let mut total_words_known = 0;

    for word in word_sequence {
        let chars = word_to_hanzi(word);
        total_chars += chars.len();
        if known_words.contains(word) {
            total_words_known += 1;
        }
    }
    BookWithStats {
        book,
        word_comprehension: total_words_known as f64 / total_words as f64,
        total_chars,
    }
}

pub struct DisplayState {
    books_with_stats: Vec<BookWithStats>,
    sort_descending: bool,
    sort_by: SortType,
}

impl DisplayState {
    pub fn new(books_with_stats: Vec<BookWithStats>) -> Self {
        Self {
            books_with_stats,
            sort_descending: true,
            sort_by: SortType::Comprehension,
        }
    }
}

pub struct BookWithStats {
    book: BookSegmentation,
    word_comprehension: f64,
    total_chars: usize,
}

pub enum SortType {
    Comprehension,
    Length,
}
