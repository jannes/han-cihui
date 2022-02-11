use crate::tui::state::info::{InfoState, SyncingState};
use crate::tui::state::TuiState;
use crate::vocabulary::VocabularyInfo;

use crossterm::event;
use crossterm::event::KeyCode;
use event::KeyEvent;

pub fn handle_event_info(
    state: &TuiState,
    current_vocab_info: &VocabularyInfo,
    key_event: KeyEvent,
) -> Option<InfoState> {
    match key_event.code {
        KeyCode::Char('s') => Some(InfoState::Syncing(SyncingState::new(
            *current_vocab_info,
            state.db_connection.clone(),
        ))),
        _ => None,
    }
}
