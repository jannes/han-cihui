use std::io;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::{env, fs};

use serde::{Deserialize, Serialize};

pub fn init_config(data_dir: &Path) {
    let config_path = data_dir.join("config.toml");
    let config = if !config_path.exists() {
        let anki_db_path = PathBuf::from(prompt(
            "please provide path to your Anki collection database file",
        ));
        let anki_notes: Vec<String> = prompt(
            "please provide note type names that should be scanned for vocabulary, \
             separated by commas on same line",
        )
        .split(',')
        .map(|s| s.to_owned())
        .collect();
        let export_base_path = PathBuf::from(prompt(
            "please provide base path the export dialog will default to",
        ));
        let config = Config {
            anki_db_path,
            anki_notes,
            export_base_path,
        };
        fs::write(
            config_path,
            toml::to_string_pretty(&config).expect("failed to serialize config"),
        )
        .expect("failed to create config.toml");
        config
    } else {
        toml::from_str(&fs::read_to_string(&config_path).expect("could not read config file"))
            .expect("malformatted config")
    };
    CONFIG.set(config).expect("global CONFIG already set");
}

pub fn get_config() -> Config {
    CONFIG.get().expect("global CONFIG not initialized").clone()
}

static CONFIG: OnceLock<Config> = OnceLock::new();

#[cfg(not(debug_assertions))]
pub const TAGGER_BIN: &str = "han-shaixuan";
#[cfg(debug_assertions)]
pub const TAGGER_BIN: &str = "target/debug/han-shaixuan";

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    pub anki_db_path: PathBuf,
    pub anki_notes: Vec<String>,
    pub export_base_path: PathBuf,
}

// making sure that when developing the path to the data directory has to be explicitely set
#[cfg(debug_assertions)]
pub fn get_data_dir() -> PathBuf {
    PathBuf::from(env::var("DATA_DIR").expect("always pass DATA_DIR env var when developing"))
}

#[cfg(not(debug_assertions))]
pub fn get_data_dir() -> PathBuf {
    match env::var("DATA_DIR") {
        Ok(path) => PathBuf::from(path),
        Err(_) => home_dir().expect("could not determine current user's home directory"),
    }
}

pub fn tagging_socket_path() -> PathBuf {
    let mut path = get_data_dir();
    path.push("sock");
    path
}

fn prompt(msg: &str) -> String {
    println!("{}:", msg);
    let mut answer = String::new();
    io::stdin()
        .read_line(&mut answer)
        .expect("error reading user input line");
    answer.trim().to_owned()
}
