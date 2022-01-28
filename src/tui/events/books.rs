use std::sync::{Arc, Mutex};

use anyhow::Result;
use crossterm::event::KeyEvent;
use rusqlite::Connection;

use crate::tui::state::books::{BooksState, CalculatingState};

pub fn handle_event_books_calculating(
    mut calculating_state: CalculatingState,
    key_event: KeyEvent,
    db: Arc<Mutex<Connection>>,
) -> Result<(BooksState, Option<String>)> {
    todo!()
}
