mod analysis;
mod info;
mod word_list;

use anyhow::Result;
use crossterm::event;
use crossterm::event::KeyCode;
use event::KeyEvent;

use self::analysis::handle_event_analysis_blank;
use self::analysis::handle_event_analysis_extracted;
use self::analysis::handle_event_analysis_opening;
use self::info::handle_event_info;
use self::word_list::handle_event_word_list_detail;
use self::word_list::handle_event_word_lists;

use super::state::analysis::AnalysisState;
use super::state::info::InfoState;
use super::state::word_list::WordListState;
use super::state::State;
use super::state::View;

pub enum Event<I> {
    Input(I),
    Tick,
}

pub(super) fn handle_event(mut state: State, event: Event<KeyEvent>) -> Result<State> {
    let key_event = match event {
        Event::Input(key_event) => {
            // handle meta shortcuts only when not currently entering input
            if !state.currently_input() {
                match key_event.code {
                    KeyCode::Char('q') => {
                        state.current_view = View::Exit;
                        return Ok(state);
                    }
                    KeyCode::Char('0') => {
                        state.current_view = View::Info;
                        return Ok(state);
                    }
                    KeyCode::Char('1') => {
                        state.current_view = View::Analysis;
                        return Ok(state);
                    }
                    KeyCode::Char('2') => {
                        state.current_view = View::WordLists;
                        return Ok(state);
                    }
                    _ => {}
                }
            }
            key_event
        }
        Event::Tick => {
            if let AnalysisState::Extracting(extracting_state) = &mut state.analysis_state {
                if let Some(new_state) = extracting_state.update() {
                    match &new_state {
                        AnalysisState::ExtractError(_) => update_action_log(
                            &mut state.action_log,
                            Some("Failed extraction".to_string()),
                        ),
                        AnalysisState::Extracted(_) => update_action_log(
                            &mut state.action_log,
                            Some("Extraction success".to_string()),
                        ),
                        _ => {}
                    }
                    state.analysis_state = new_state;
                }
            }
            if let InfoState::Syncing(syncing_state) = &mut state.info_state {
                if let Some(new_state) = syncing_state.update() {
                    // if sync successfully completed, add msg to action log
                    if let InfoState::Display(display_state) = &new_state {
                        // since it's newly updated, must have Some(previous_vocab_info)
                        let (active_words_diff, active_known_chars_diff) = display_state.get_diff_active_words_chars()
                            .expect("newly synced display state should have Some(previous_vocab_info) field");
                        state.action_log.push(format!(
                            "synced Anki: {} new words, {} new chars",
                            active_words_diff, active_known_chars_diff
                        ));
                        state.info_state = new_state;
                    }
                }
            }
            return Ok(state);
        }
    };

    match state.current_view {
        View::Analysis => {
            state.analysis_state = match state.analysis_state {
                AnalysisState::Extracted(extracted_state) => {
                    let (new_state, action) = handle_event_analysis_extracted(
                        extracted_state,
                        key_event,
                        state.db_connection.clone(),
                    )?;
                    update_action_log(&mut state.action_log, action);
                    new_state
                }
                AnalysisState::Opening(partial_path, seg_mode) => {
                    let (new_state, action) = handle_event_analysis_opening(
                        partial_path,
                        key_event,
                        seg_mode,
                        state.db_connection.clone(),
                    );
                    update_action_log(&mut state.action_log, action);
                    new_state
                }
                AnalysisState::Blank => handle_event_analysis_blank(key_event),
                AnalysisState::ExtractError(_) => handle_event_analysis_blank(key_event),
                x => x,
            }
        }
        View::Info => {
            let new_state = match &state.info_state {
                // allow no tab specific actions during syncing
                InfoState::Syncing(_) => None,
                InfoState::Display(display_state) => {
                    handle_event_info(&state, &display_state.vocab_info, key_event)
                }
                InfoState::SyncError(sync_error_state) => {
                    // TODO: switch back to display state if no new state comes up + append error msg to action log
                    handle_event_info(&state, &sync_error_state.previous_vocab_info, key_event)
                }
            };
            // switched to syncing state
            if let Some(new_state) = new_state {
                state.info_state = new_state;
            }
        }
        View::WordLists => {
            let new_state = match &state.word_list_state {
                WordListState::ListOfWordLists { word_lists } => {
                    handle_event_word_lists(&state, key_event)
                }
                WordListState::OpenedWordList { word_list } => {
                    handle_event_word_list_detail(&state, key_event)
                }
            };
        }
        View::Exit => {}
    };
    Ok(state)
}

fn update_action_log(action_log: &mut Vec<String>, action: Option<String>) {
    if let Some(action) = action {
        action_log.push(action);
    }
}
