use std::sync::{Arc, Mutex};

use crossterm::event::{KeyCode, KeyEvent};
use rusqlite::Connection;

use crate::{
    db::books::db_books_delete,
    ebook::open_as_flat_book,
    tui::state::books::{BooksState, DisplayState, ImportingState},
};

pub fn handle_event_books_display(
    mut state: DisplayState,
    key_event: KeyEvent,
    db: Arc<Mutex<Connection>>,
) -> (BooksState, Option<String>) {
    match key_event.code {
        KeyCode::Char('i') => (BooksState::EnterToImport("".to_string()), None),
        KeyCode::Char('j') => {
            state.select_next();
            (BooksState::Display(state), None)
        }
        KeyCode::Char('k') => {
            state.select_previous();
            (BooksState::Display(state), None)
        }
        KeyCode::Char('d') => {
            if let Some(book) = state.remove_current() {
                let action = match db_books_delete(&db.lock().unwrap(), &book.title, &book.author) {
                    Ok(_) => Some(format!("deleted {}", book.title)),
                    Err(e) => Some(format!("deletion failed, err: {:?}", e)),
                };
                return (BooksState::Display(state), action);
            }
            (BooksState::Display(state), None)
        }
        _ => (BooksState::Display(state), None),
    }
}

pub fn handle_event_books_enter_to_import(
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
