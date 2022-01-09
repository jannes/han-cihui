use crate::tui::state::word_list::WordListState;
use crate::tui::state::State;

use crossterm::event;
use crossterm::event::KeyCode;
use event::KeyEvent;

pub fn handle_event_word_lists(state: &State, key_event: KeyEvent) -> Option<WordListState> {
    match key_event.code {
        KeyCode::Enter => todo!(),
        KeyCode::Char('j') => None,
        KeyCode::Char('k') => None,
        _ => None,
    }
}

pub fn handle_event_word_list_detail(state: &State, key_event: KeyEvent) -> Option<WordListState> {
    todo!()
}
