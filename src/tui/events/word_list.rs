use std::sync::{Arc, Mutex};

use crate::db::word_lists::db_wlist_delete_by_id;
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
) -> Result<(WordListState, Option<String>)> {
    match key_event.code {
        KeyCode::Enter => {
            return Ok((state.try_open(&db_conn.lock().unwrap())?, None));
        }
        KeyCode::Char('j') => {
            state.select_next();
        }
        KeyCode::Char('k') => {
            state.select_previous();
        }
        KeyCode::Char('d') => {
            if let Some(wlm) = state.remove_current() {
                let action = match db_wlist_delete_by_id(&db_conn.lock().unwrap(), wlm.id) {
                    Ok(_) => Some(format!("deleted wlist for {}", wlm.book_name)),
                    Err(e) => Some(format!("deletion failed, err: {:?}", e)),
                };
                return Ok((WordListState::List(state), action));
            };
        }
        _ => {}
    }
    Ok((WordListState::List(state), None))
}

pub fn handle_event_word_list_opened(
    key_event: KeyEvent,
    mut state: OpenedWordList,
    db: Arc<Mutex<Connection>>,
) -> Result<WordListState> {
    match key_event.code {
        KeyCode::Enter => {
            if let Some(selected_chapter) = state.get_selected_mut() {
                selected_chapter.modify_tw(|tagged_words| todo!());
            }
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
        KeyCode::Char('e') => {
            todo!()
        }
        _ => {}
    }
    Ok(WordListState::Opened(state))
}
