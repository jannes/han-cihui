use crate::state::AnalysisState;
use crate::{
    analysis::{get_filtered_extraction_items, save_filtered_extraction_info},
    extraction::ExtractionItem,
    segmentation::SegmentationMode,
    state::{ExtractQuery, ExtractedSavingState, ExtractedState, ExtractingState},
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

pub fn handle_event_analysis_extracted(
    mut extracted_state: ExtractedState,
    key_event: KeyEvent,
) -> AnalysisState {
    let mut analysis_query = extracted_state.analysis_query;
    match key_event.code {
        KeyCode::Char('r') => return AnalysisState::Blank,
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

pub fn handle_event_analysis_saving(
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

pub fn handle_event_analysis_opening(
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

pub fn handle_event_analysis_blank(key_event: KeyEvent) -> AnalysisState {
    match key_event.code {
        KeyCode::Char('e') => {
            AnalysisState::Opening("".to_string(), SegmentationMode::DictionaryOnly)
        }
        _ => AnalysisState::Blank,
    }
}
