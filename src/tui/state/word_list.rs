use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::{
    persistence::select_all_word_lists_metadata,
    word_lists::{WordList, WordListMetadata},
};

pub enum WordListState {
    ListOfWordLists { word_lists: Vec<WordListMetadata> },
    OpenedWordList { word_list: WordList },
}

impl WordListState {
    pub fn init(db: Arc<Mutex<Connection>>) -> Result<Self> {
        let word_lists = select_all_word_lists_metadata(&db.lock().unwrap())
            .context("unable to load word lists")?;
        Ok(WordListState::ListOfWordLists { word_lists })
    }
}
