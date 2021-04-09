use crate::{
    analysis::{get_filtered_extraction_items, save_filtered_extraction_info},
    extraction::ExtractionItem,
    segmentation::SegmentationMode,
    state::{
        AnalysisState, ExtractQuery, ExtractedSavingState, ExtractedState, ExtractingState,
        InfoState, State, SyncingState, View,
    },
    vocabulary::VocabularyInfo,
};
use anyhow::Result;
use crossterm::event;
use crossterm::event::KeyCode;
use event::KeyEvent;
use rusqlite::Connection;
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

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
                    handle_event_analysis_extracted(extracted_state, key_event)
                }
                AnalysisState::ExtractedSaving(extracted_saving_state) => {
                    let (new_state, action) =
                        handle_event_analysis_saving(extracted_saving_state, key_event)?;
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
        View::Exit => {}
    };
    Ok(state)
}

fn update_action_log(action_log: &mut Vec<String>, action: Option<String>) {
    if let Some(action) = action {
        action_log.push(action);
    }
}

fn handle_event_analysis_extracted(
    mut extracted_state: ExtractedState,
    key_event: KeyEvent,
) -> AnalysisState {
    let mut analysis_query = extracted_state.analysis_query;
    match key_event.code {
        KeyCode::Char('s') => {
            return AnalysisState::ExtractedSaving(ExtractedSavingState {
                extracted_state,
                partial_save_path: String::new(),
            })
        }
        // reduce min_occurrence of words
        KeyCode::Char('j') => {
            analysis_query.min_occurrence_words = *analysis_query
                .min_occurrence_words
                .checked_sub(1)
                .get_or_insert(0);
        }
        // increase min_occurrence of words
        KeyCode::Char('k') => {
            analysis_query.min_occurrence_words += 1;
        }
        // reduce min_occurrence of unknown chars
        KeyCode::Char('h') => {
            analysis_query.min_occurrence_unknown_chars =
                match analysis_query.min_occurrence_unknown_chars {
                    Some(amount) => {
                        if amount == 1 {
                            None
                        } else {
                            Some(amount.checked_sub(1).unwrap())
                        }
                    }
                    None => None,
                };
        }
        // increase min_occurrence of unknown chars
        KeyCode::Char('l') => {
            analysis_query.min_occurrence_unknown_chars =
                match extracted_state.analysis_query.min_occurrence_unknown_chars {
                    Some(amount) => Some(amount + 1),
                    None => Some(1),
                }
        }
        _ => {}
    }
    extracted_state.query_update(analysis_query);
    AnalysisState::Extracted(extracted_state)
}

fn handle_event_analysis_saving(
    mut saving_state: ExtractedSavingState,
    key_event: KeyEvent,
) -> Result<(AnalysisState, Option<String>)> {
    match key_event.code {
        KeyCode::Char(c) => {
            saving_state.partial_save_path.push(c);
        }
        KeyCode::Backspace => {
            saving_state.partial_save_path.pop();
        }
        KeyCode::Esc => {
            return Ok((
                AnalysisState::Extracted(saving_state.extracted_state),
                Some("canceled save".to_string()),
            ));
        }
        KeyCode::Enter => {
            let extracted = &saving_state.extracted_state;
            let book = &extracted.book;
            let filtered_extraction_set = get_filtered_extraction_items(
                &extracted.extraction_result,
                extracted.analysis_query.min_occurrence_words,
                extracted.get_known_words(),
                extracted.analysis_query.min_occurrence_unknown_chars,
            );
            let known_words = saving_state.extracted_state.get_known_words();
            let unknown_to_save: HashSet<&ExtractionItem> = filtered_extraction_set
                .into_iter()
                .filter(|item| !known_words.contains(&item.word))
                .collect();
            save_filtered_extraction_info(book, &unknown_to_save, &saving_state.partial_save_path)?;
            return Ok((
                AnalysisState::Extracted(saving_state.extracted_state),
                Some(format!("saved to {}", &saving_state.partial_save_path)),
            ));
        }
        _ => {}
    }
    Ok((AnalysisState::ExtractedSaving(saving_state), None))
}

fn handle_event_analysis_opening(
    mut partial_path: String,
    key_event: KeyEvent,
    seg_mode: SegmentationMode,
    db: Arc<Mutex<Connection>>,
) -> (AnalysisState, Option<String>) {
    match key_event.code {
        KeyCode::Char(c) => {
            partial_path.push(c);
        }
        KeyCode::Backspace => {
            partial_path.pop();
        }
        KeyCode::Esc => {
            return (AnalysisState::Blank, Some("canceled open".to_string()));
        }
        KeyCode::Enter => {
            let extract_query = ExtractQuery {
                filename: partial_path.clone(),
                segmentation_mode: seg_mode,
            };
            let analysis_state = AnalysisState::Extracting(ExtractingState::new(extract_query, db));
            return (
                analysis_state,
                Some(format!("opening {} for analysis", partial_path)),
            );
        }
        _ => {}
    }
    (AnalysisState::Opening(partial_path, seg_mode), None)
}

fn handle_event_analysis_blank(key_event: KeyEvent) -> AnalysisState {
    match key_event.code {
        KeyCode::Char('e') => {
            AnalysisState::Opening("".to_string(), SegmentationMode::DictionaryOnly)
        }
        _ => AnalysisState::Blank,
    }
}

fn handle_event_info(
    state: &State,
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
