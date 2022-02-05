use std::sync::{Arc, Mutex};

use crate::tui::state::word_list::{ListOfWordLists, OpenedWordList, WordListState};

use crossterm::event;
use crossterm::event::KeyCode;
use event::KeyEvent;
use rusqlite::Connection;

pub fn handle_event_word_lists(
    key_event: KeyEvent,
    mut state: ListOfWordLists,
    db: Arc<Mutex<Connection>>,
) -> WordListState {
    match key_event.code {
        KeyCode::Enter => todo!(),
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
    WordListState::List(state)
}

pub fn handle_event_word_list_detail(
    key_event: KeyEvent,
    state: OpenedWordList,
    db: Arc<Mutex<Connection>>,
) -> WordListState {
    todo!()
}
