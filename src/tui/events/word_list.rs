use std::sync::{Arc, Mutex};

use crate::tui::state::word_list::WordListState;

use crossterm::event;
use crossterm::event::KeyCode;
use event::KeyEvent;
use rusqlite::Connection;

pub fn handle_event_word_lists(
    key_event: KeyEvent,
    state: WordListState,
    db: Arc<Mutex<Connection>>,
) -> WordListState {
    match state {
        WordListState::ListOfWordLists {
            word_lists,
            selected,
        } => {
            let selected = match key_event.code {
                KeyCode::Enter => todo!(),
                KeyCode::Char('j') => match selected {
                    Some(i) if i < word_lists.len() - 1 => Some(i + 1),
                    None => Some(0),
                    _ => selected,
                },
                KeyCode::Char('k') => match selected {
                    Some(i) if i > 0 => Some(i - 1),
                    None => Some(0),
                    _ => selected,
                },
                _ => selected,
            };
            WordListState::ListOfWordLists {
                word_lists,
                selected,
            }
        }
        _ => unreachable!(),
    }
}

pub fn handle_event_word_list_detail(
    key_event: KeyEvent,
    state: WordListState,
    db: Arc<Mutex<Connection>>,
) -> WordListState {
    todo!()
}
