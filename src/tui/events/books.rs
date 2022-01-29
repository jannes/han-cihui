use std::sync::{Arc, Mutex};

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use rusqlite::Connection;

use crate::{
    ebook::open_as_flat_book,
    tui::state::books::{BooksState, CalculatingState, ImportingState},
};

pub fn handle_event_books_calculating(
    mut calculating_state: CalculatingState,
    key_event: KeyEvent,
    db: Arc<Mutex<Connection>>,
) -> Result<(BooksState, Option<String>)> {
    todo!()
}

pub fn handle_event_books_importing(
    mut partial_path: String,
    key_event: KeyEvent,
    db: Arc<Mutex<Connection>>,
) -> (BooksState, Option<String>) {
    match key_event.code {
        KeyCode::Char(c) => {
            partial_path.push(c);
        }
        KeyCode::Backspace => {
            partial_path.pop();
        }
        KeyCode::Esc => {
            return (BooksState::Uninitialized, Some("canceled open".to_string()));
        }
        KeyCode::Enter => match open_as_flat_book(&partial_path, 1) {
            Ok(b) => {
                let action = Some(format!("imported {} by {}", &b.title, &b.author));
                return (BooksState::Importing(ImportingState::new(b, db)), action);
            }
            Err(e) => {
                return (
                    BooksState::Uninitialized,
                    Some(format!("failed import: {}", e)),
                )
            }
        },
        _ => {}
    }
    (BooksState::EnterToImport(partial_path), None)
}
