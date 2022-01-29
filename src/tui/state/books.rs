use anyhow::Result;
use std::{
    cell::RefCell,
    collections::HashSet,
    sync::{
        mpsc::{self, Receiver},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use tui::widgets::TableState;

use rusqlite::Connection;

use crate::{
    ebook::FlatBook,
    extraction::word_to_hanzi,
    persistence::{db_books_insert, db_books_select_all, db_words_select_known},
    segmentation::{segment_book, BookSegmentation, SegmentationMode},
};

pub enum BooksState {
    Uninitialized,
    Calculating(CalculatingState),
    Display(DisplayState),
    // String arg: partial file path
    EnterToImport(String),
    Importing(ImportingState),
}

impl BooksState {
    pub fn init(db_connection: Arc<Mutex<Connection>>) -> Result<Self> {
        let books = db_books_select_all(&db_connection.lock().unwrap())?;
        let known_words = db_words_select_known(&db_connection.lock().unwrap())?;
        Ok(Self::Calculating(CalculatingState::new(books, known_words)))
    }

    pub fn is_init(&self) -> bool {
        if let BooksState::Uninitialized = self {
            false
        } else {
            true
        }
    }
}

pub struct CalculatingState {
    // (title, author, book)
    pub books: Vec<(String, String, BookSegmentation)>,
    pub known_words: HashSet<String>,
    pub start: Instant,
}

impl CalculatingState {
    pub fn new(
        books: Vec<(String, String, BookSegmentation)>,
        known_words: HashSet<String>,
    ) -> Self {
        Self {
            books,
            known_words,
            start: Instant::now(),
        }
    }

    pub fn update(&self) -> BooksState {
        let mut books_with_stats = Vec::with_capacity(self.books.len());
        for (title, author, book) in &self.books {
            books_with_stats.push(get_enrich_book_with_stats(
                title.clone(),
                author.clone(),
                book.clone(),
                &self.known_words,
            ))
        }
        BooksState::Display(DisplayState::new(books_with_stats))
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}

pub fn get_enrich_book_with_stats(
    title: String,
    author: String,
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
        title,
        author,
    }
}

pub struct DisplayState {
    pub books_with_stats: Vec<BookWithStats>,
    pub sort_descending: bool,
    pub sort_by: SortType,
    pub table_state: RefCell<TableState>,
}

impl DisplayState {
    pub fn new(books_with_stats: Vec<BookWithStats>) -> Self {
        Self {
            books_with_stats,
            sort_descending: true,
            sort_by: SortType::Comprehension,
            table_state: RefCell::new(TableState::default()),
        }
    }

    pub fn next(&mut self) {
        let i = match self.table_state.borrow().selected() {
            Some(i) => {
                if i >= self.books_with_stats.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.borrow_mut().select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.table_state.borrow().selected() {
            Some(i) => {
                if i == 0 {
                    self.books_with_stats.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.borrow_mut().select(Some(i));
    }
}

pub struct ImportingState {
    pub book_title: String,
    pub book_author: String,
    pub receiver: Receiver<BookSegmentation>,
    pub segmenter_thread: JoinHandle<()>,
    pub db_connection: Arc<Mutex<Connection>>,
    pub start: Instant,
}

impl ImportingState {
    pub fn new(book: FlatBook, db_connection: Arc<Mutex<Connection>>) -> Self {
        let book_title = book.title.clone();
        let book_author = book.author.clone();
        let (tx, rx) = mpsc::channel();
        let segmenter_thread = thread::spawn(move || {
            let res = segment_book(&book, SegmentationMode::Default);
            tx.send(res).expect("could not send event");
        });
        ImportingState {
            receiver: rx,
            segmenter_thread,
            start: Instant::now(),
            book_title,
            book_author,
            db_connection,
        }
    }

    // TODO: improve error handling
    // update state, return new state if extraction thread terminated, otherwise return None
    pub fn update(&mut self) -> Option<(BooksState, String)> {
        match self.receiver.try_recv() {
            Ok(segmented_book) => {
                // save book
                let action = match db_books_insert(
                    &self.db_connection.lock().unwrap(),
                    &self.book_title,
                    &self.book_author,
                    &segmented_book,
                ) {
                    Ok(_) => format!("saved {}", self.book_title),
                    Err(e) => format!("error saving {}: {}", self.book_title, e),
                };
                Some((BooksState::Uninitialized, action))
            }
            Err(e) => match e {
                mpsc::TryRecvError::Empty => None,
                mpsc::TryRecvError::Disconnected => Some((
                    BooksState::Uninitialized,
                    "Segmentation manager thread disconnected".to_string(),
                )),
            },
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}

pub struct BookWithStats {
    pub title: String,
    pub author: String,
    pub book: BookSegmentation,
    pub word_comprehension: f64,
    pub total_chars: usize,
}

pub enum SortType {
    Comprehension,
    Length,
}
