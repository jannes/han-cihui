use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::{
    persistence::db_wlist_select_all_mdata,
    word_lists::{WordList, WordListMetadata},
};

pub enum WordListState {
    ListOfWordLists {
        word_lists: Vec<WordListMetadata>,
        selected: Option<usize>,
    },
    OpenedWordList {
        word_list: WordList,
    },
}

impl WordListState {
    pub fn init(db: Arc<Mutex<Connection>>) -> Result<Self> {
        let word_lists =
            db_wlist_select_all_mdata(&db.lock().unwrap()).context("unable to load word lists")?;
        let selected = None;
        Ok(WordListState::ListOfWordLists {
            word_lists,
            selected,
        })
    }
}
