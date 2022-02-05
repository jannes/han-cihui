extern crate clap;
#[macro_use]
extern crate lazy_static;
extern crate rusqlite;

use crate::tui::state::State;
use crate::tui::TuiApp;
use cli::{get_arg_matches, perform_add_external, perform_delete_external};
use db::vocab::AddedExternal;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::{env, fs};

use anyhow::Result;

mod analysis;
mod cli;
mod db;
mod ebook;
mod extraction;
mod segmentation;
mod tui;
mod vocabulary;
mod word_lists;

#[cfg(not(debug_assertions))]
const DATA_DIR: &str = "/Users/jannes/.han-cihui";

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
        _ => TuiApp::new_stdout(State::new(data_conn)?)?.run(),
    }
}
