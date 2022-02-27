use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::config::EXPORT_BASE_PATH;
use crate::db::word_lists::{db_wlist_delete_by_id, db_wlist_update};
use crate::tui::state::word_list::{ListOfWordLists, OpenedWordList, WordListState};
use crate::word_lists::tag_words;
use anyhow::{Context, Result};

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
) -> Result<(WordListState, Option<String>)> {
    let mut action = None;
    match key_event.code {
        KeyCode::Enter => {
            if let Some((_, selected_chapter)) = state.get_selected_mut() {
                selected_chapter.modify_tw(tag_words);
                state.sync();
                db_wlist_update(
                    &db.lock().unwrap(),
                    state.word_list_id(),
                    &state.get_chapter_words(),
                )?;
            }
        }
        KeyCode::Esc => {
            return WordListState::init(db).map(|state| (state, None));
        }
        KeyCode::Char('j') => {
            state.select_next();
        }
        KeyCode::Char('k') => {
            state.select_previous();
        }
        KeyCode::Char('e') => {
            if let Some((i, chapter_info)) = state.get_selected() {
                let words_to_learn = chapter_info
                    .get_words_to_learn()
                    .iter()
                    .fold("".to_string(), |s, w| format!("{}{}\n", s, w));
                let chapter_title = chapter_info.chapter_title();
                let wlist_metadata = state.word_list_metadata();
                let mut p = PathBuf::from(format!("{}/{}", EXPORT_BASE_PATH, wlist_metadata));
                if !p.exists() {
                    fs::create_dir(&p).context("could not create folder")?;
                }
                let filename = format!("{}{}-{}.txt", wlist_metadata.book_name, i, chapter_title);
                p.push(&filename);
                fs::write(&p, words_to_learn).context("could not write words to leanr")?;
                action = Some(format!("{} exported", &filename));
            }
        }
        _ => {}
    }
    Ok((WordListState::Opened(state), action))
}
