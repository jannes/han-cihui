use han_cihui::cli::{get_arg_matches, perform_add_external, perform_delete_external, show};
use han_cihui::config::get_data_dir;
use han_cihui::db::vocab::AddedExternal;
use han_cihui::tui::state::TuiState;
use han_cihui::tui::TuiApp;
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;

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
        Some("show") => show(&data_conn),
        _ => TuiApp::new_stdout(TuiState::new(data_conn)?)?.run(),
    }
}
