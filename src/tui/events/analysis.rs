use crate::{
    db::word_lists::db_wlist_insert,
    extraction::ExtractionItem,
    tui::state::analysis::{AnalysisState, ExtractedState},
    word_lists::construct_word_list,
};
use anyhow::{Context, Result};
use crossterm::event;
use crossterm::event::KeyCode;
use event::KeyEvent;
use rusqlite::Connection;
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

pub fn handle_event_analysis(
    mut extracted_state: Box<ExtractedState>,
    key_event: KeyEvent,
    db: Arc<Mutex<Connection>>,
) -> Result<(AnalysisState, Option<String>)> {
    let mut analysis_query = extracted_state.analysis_query;
    let mut action_log_entry: Option<String> = None;
    match key_event.code {
        KeyCode::Char('r') => return Ok((AnalysisState::Blank, None)),
        KeyCode::Char('s') => {
            let book = &extracted_state.extraction_result.segmented_book;
            let analysis_query = extracted_state.analysis_query;
            let unknown_words_to_save: HashSet<&ExtractionItem> = extracted_state
                .extraction_result
                .vocabulary
                .iter()
                .collect();
            let word_list = construct_word_list(book, analysis_query, &unknown_words_to_save);
            db_wlist_insert(&db.lock().unwrap(), word_list)
                .context("unable to save word list to DB")?;
            action_log_entry = Some(format!("Saved word list for {}", book.title_cut.join("")));
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
    Ok((AnalysisState::Extracted(extracted_state), action_log_entry))
}
