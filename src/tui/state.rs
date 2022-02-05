pub mod analysis;
pub mod books;
pub mod info;
pub mod word_list;

use anyhow::Result;
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use self::{analysis::AnalysisState, books::BooksState, info::InfoState, word_list::WordListState};

pub struct State {
    pub analysis_state: AnalysisState,
    pub books_state: BooksState,
    pub info_state: InfoState,
    pub word_list_state: WordListState,
    pub current_view: View,
    pub db_connection: Arc<Mutex<Connection>>,
    pub action_log: Vec<String>,
}

impl State {
    pub fn new(db_connection: Connection) -> Result<Self> {
        let db_connection = Arc::new(Mutex::new(db_connection));
        Ok(State {
            analysis_state: AnalysisState::default(),
            books_state: BooksState::init(db_connection.clone())?,
            info_state: InfoState::init(db_connection.clone())?,
            word_list_state: WordListState::init(db_connection.clone())?,
            current_view: View::Info,
            db_connection,
            action_log: vec![],
        })
    }

    /// Is the user currently entering something in an input box?
    pub fn currently_input(&self) -> bool {
        match self.current_view {
            View::Books => matches!(self.books_state, BooksState::EnterToImport(..)),
            _ => false,
        }
    }
}

pub enum View {
    Info,
    Books,
    Analysis,
    WordLists,
    Exit,
}
