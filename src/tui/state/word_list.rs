use std::{
    cell::RefCell,
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result};
use rusqlite::Connection;
use tui::widgets::TableState;

use crate::{
    db::word_lists::db_wlist_select_all_mdata,
    word_lists::{WordList, WordListMetadata},
};

pub enum WordListState {
    List(ListOfWordLists),
    Opened(OpenedWordList),
}

impl WordListState {
    pub fn init(db: Arc<Mutex<Connection>>) -> Result<Self> {
        let word_lists =
            db_wlist_select_all_mdata(&db.lock().unwrap()).context("unable to load word lists")?;
        let table_state = RefCell::new(TableState::default());
        Ok(WordListState::List(ListOfWordLists {
            word_lists,
            table_state,
        }))
    }
}

pub struct ListOfWordLists {
    pub word_lists: Vec<WordListMetadata>,
    pub table_state: RefCell<TableState>,
}

impl ListOfWordLists {
    pub fn select_next(&mut self) {
        if self.word_lists.is_empty() {
            return;
        }
        let i = match self.table_state.borrow().selected() {
            Some(i) => {
                if i >= self.word_lists.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.borrow_mut().select(Some(i));
    }

    pub fn select_previous(&mut self) {
        if self.word_lists.is_empty() {
            return;
        }
        let i = match self.table_state.borrow().selected() {
            Some(i) => {
                if i == 0 {
                    self.word_lists.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.borrow_mut().select(Some(i));
    }
}

pub struct OpenedWordList {
    pub word_list: WordList,
    pub table_state: RefCell<TableState>,
}
