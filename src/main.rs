extern crate clap;
#[macro_use]
extern crate lazy_static;
extern crate rusqlite;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::{env, fs};

use crate::tui::state::analysis::{AnalysisState, ExtractQuery, ExtractingState};
use crate::tui::state::info::InfoState;
use crate::tui::state::{State, View};
use crate::tui::TuiApp;
use cli_commands::{perform_add_external, perform_delete_external, print_anki_stats, show};
use rusqlite::Connection;

use crate::cli_args::get_arg_matches;
use crate::persistence::AddedExternal;
use crate::segmentation::SegmentationMode;
use anyhow::Result;

mod analysis;
mod anki_access;
mod cli_args;
mod cli_commands;
mod ebook;
mod extraction;
mod persistence;
mod segmentation;
mod tui;
mod vocabulary;
mod word_lists;

pub const WORD_DELIMITERS: [char; 3] = ['/', '\\', ' '];
pub const NOTE_FIELD_PAIRS: [(&str, &str); 1] = [("中文-英文", "中文")];

#[cfg(not(debug_assertions))]
const DATA_DIR: &str = "/Users/jannes/.han-cihui";
const ANKIDB_PATH: &str = "/Users/jannes/Library/ApplicationSupport/Anki2/Jannes/collection.anki2";

// making sure that when developing the path to the data directory has to be explicitely set
#[cfg(debug_assertions)]
fn get_data_dir() -> String {
    env::var("DATA_DIR").expect("always pass DATA_DIR env var when developing")
}

#[cfg(not(debug_assertions))]
fn get_data_dir() -> String {
    match env::var("DATA_DIR") {
        Ok(s) => s,
        Err(_) => DATA_DIR.to_string(),
    }
}

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("migrations_sql");
}

fn main() -> Result<()> {
    let data_dir = PathBuf::from(get_data_dir());
    if !data_dir.exists() {
        // if first time call, create data directory
        println!(
            "performing first time setup, creating {}",
            data_dir.display()
        );
        fs::create_dir(&data_dir)?;
    }
    let db_path: PathBuf = [data_dir.as_path(), Path::new("data.db")].iter().collect();
    let mut data_conn = Connection::open(db_path)?;
    embedded::migrations::runner().run(&mut data_conn)?;

    let matches = get_arg_matches();
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
