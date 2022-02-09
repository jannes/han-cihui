use std::env;

#[cfg(not(debug_assertions))]
const DATA_DIR: &str = "/Users/jannes/.han-cihui";

pub const ANKIDB_PATH: &str =
    "/Users/jannes/Library/ApplicationSupport/Anki2/Jannes/collection.anki2";
pub const ANKI_NOTE_FIELD_PAIRS: [(&str, &str); 1] = [("中文-英文", "中文")];
pub const ANKI_WORD_DELIMITERS: [char; 3] = ['/', '\\', ' '];
pub const ANKI_SUSPENDED_KNOWN_FLAG: i32 = 3; // green
pub const ANKI_SUSPENDED_UNKNOWN_FLAG: i32 = 0; // no flag

pub const DEFAULT_CHAPTERS_DEPTH: u32 = 2;

// making sure that when developing the path to the data directory has to be explicitely set
#[cfg(debug_assertions)]
pub fn get_data_dir() -> String {
    env::var("DATA_DIR").expect("always pass DATA_DIR env var when developing")
}

#[cfg(not(debug_assertions))]
pub fn get_data_dir() -> String {
    match env::var("DATA_DIR") {
        Ok(s) => s,
        Err(_) => DATA_DIR.to_string(),
    }
}
