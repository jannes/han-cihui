use std::env;

pub const ANKIDB_PATH: &str =
    "/Users/jannes/Library/ApplicationSupport/Anki2/Jannes/collection.anki2";
pub const ANKI_NOTE_FIELD_PAIRS: [(&str, &str); 1] = [("中文-英文", "中文")];
pub const ANKI_SUSPENDED_KNOWN_FLAG: i32 = 3; // green
pub const ANKI_SUSPENDED_UNKNOWN_FLAG: i32 = 0; // no flag

pub const EXPORT_BASE_PATH: &str = "/Users/jannes/Nextcloud/中文/小说生词";

pub const TAGGING_SOCKET_PATH: &str = "/Users/jannes/.han-cihui/sock";
#[cfg(not(debug_assertions))]
pub const TAGGER_BIN: &str = "han-shaixuan";
#[cfg(debug_assertions)]
pub const TAGGER_BIN: &str = "target/debug/han-shaixuan";

#[cfg(not(debug_assertions))]
const DATA_DIR: &str = "/Users/jannes/.han-cihui";

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
