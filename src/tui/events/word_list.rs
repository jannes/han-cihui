use std::sync::{Arc, Mutex};

use crate::tui::state::word_list::{ListOfWordLists, OpenedWordList, WordListState};
use anyhow::Result;

use crossterm::event;
use crossterm::event::KeyCode;
use event::KeyEvent;
use rusqlite::Connection;

pub fn handle_event_word_lists(
    key_event: KeyEvent,
    mut state: ListOfWordLists,
    db_conn: Arc<Mutex<Connection>>,
) -> Result<WordListState> {
    match key_event.code {
        KeyCode::Enter => {
            return state.try_open(&db_conn.lock().unwrap());
        }
        KeyCode::Char('j') => {
            state.select_next();
        }
        KeyCode::Char('k') => {
            state.select_previous();
        }
        KeyCode::Char('d') => {
            todo!()
        }
        _ => {}
    }
    Ok(WordListState::List(state))
}

pub fn handle_event_word_list_opened(
    key_event: KeyEvent,
    mut state: OpenedWordList,
    db: Arc<Mutex<Connection>>,
) -> Result<WordListState> {
    match key_event.code {
        KeyCode::Enter => {
            todo!()
        }
        KeyCode::Esc => {
            return WordListState::init(db);
        }
        KeyCode::Char('j') => {
            state.select_next();
        }
        KeyCode::Char('k') => {
            state.select_previous();
        }
        KeyCode::Char('s') => {
            todo!()
        }
        _ => {}
    }
    Ok(WordListState::Opened(state))
}
