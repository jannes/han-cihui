extern crate clap;
#[macro_use]
extern crate lazy_static;
extern crate rusqlite;

use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::tui::TuiApp;
use cli_commands::{perform_add_external, perform_delete_external, print_anki_stats, show};
use rusqlite::Connection;
use state::{ExtractQuery, ExtractingState};

use crate::cli_args::get_arg_matches;
use crate::persistence::{create_table, AddedExternal};
use crate::segmentation::SegmentationMode;
use crate::state::{AnalysisState, InfoState, State, View};
use anyhow::Result;

mod analysis;
mod anki_access;
mod cli_args;
mod cli_commands;
mod ebook;
mod extraction;
mod persistence;
mod segmentation;
mod state;
mod tui;
mod vocabulary;

pub const WORD_DELIMITERS: [char; 3] = ['/', '\\', ' '];
pub const NOTE_FIELD_PAIRS: [(&str, &str); 1] = [("中文-英文", "中文")];
pub const SUSPENDED_KNOWN_FLAG: i32 = 3;
// green
pub const SUSPENDED_UNKNOWN_FLAG: i32 = 0; // no flag

const DATA_DIR: &str = "/Users/jannes/.zhvocab";
const DATA_PATH: &str = "/Users/jannes/.zhvocab/data.db";
const ANKIDB_PATH: &str = "/Users/jannes/Library/ApplicationSupport/Anki2/Jannes/collection.anki2";

fn main() -> Result<()> {
    let matches = get_arg_matches();

    let data_conn: Connection;
    // if first time call, do data setup
    if !Path::new(DATA_PATH).exists() {
        println!("performing first time setup");
        data_conn = first_time_setup()?;
    } else {
        data_conn = Connection::open(DATA_PATH)?;
    }

    match matches.subcommand_name() {
        Some("add") => {
            let matches = matches.subcommand_matches("add").unwrap();
            let filename = matches.value_of("filename").unwrap();
            perform_add_external(&data_conn, filename, AddedExternal::Known)
        }
        Some("delete") => {
            let matches = matches.subcommand_matches("delete").unwrap();
            let filename = matches.value_of("filename").unwrap();
            perform_delete_external(&data_conn, filename)
        }
        Some("add-ignore") => {
            let matches = matches.subcommand_matches("add-ignore").unwrap();
            let filename = matches.value_of("filename").unwrap();
            perform_add_external(&data_conn, filename, AddedExternal::Ignored)
        }
        Some("show") => {
            let matches = matches.subcommand_matches("show").unwrap();
            show(matches, &data_conn)
        }
        Some("anki-stats") => print_anki_stats(&data_conn),
        Some("analyze") => {
            let subcommand_matches = matches.subcommand_matches("analyze").unwrap();
            let filename = subcommand_matches.value_of("filename").unwrap();
            let segmentation_mode = if subcommand_matches.is_present("dict-only") {
                SegmentationMode::DictionaryOnly
            } else {
                SegmentationMode::Default
            };
            let extract_query = ExtractQuery {
                filename: filename.to_string(),
                segmentation_mode,
            };
            let db = Arc::new(Mutex::new(data_conn));
            let state = State {
                analysis_state: AnalysisState::Extracting(ExtractingState::new(
                    extract_query,
                    db.clone(),
                )),
                info_state: InfoState::init(db.clone())?,
                current_view: View::Analysis,
                db_connection: db,
                action_log: vec![],
            };
            TuiApp::new_stdout(state)?.run()
        }
        _ => TuiApp::new_stdout(State::new(data_conn)?)?.run(),
    }
}

/// perform first time setup: create sqlite database and words table
/// return database connection
fn first_time_setup() -> Result<Connection> {
    fs::create_dir(DATA_DIR)?;
    let conn = Connection::open(DATA_PATH)?;
    create_table(&conn)?;
    Ok(conn)
}
