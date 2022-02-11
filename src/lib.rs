#[macro_use]
extern crate lazy_static;

pub mod analysis;
pub mod cli;
pub mod config;
pub mod db;
pub mod ebook;
pub mod extraction;
pub mod segmentation;
pub mod tui;
pub mod vocabulary;
pub mod word_lists;

// lazy_static! {
//     static ref DB: Mutex<Option<Connection>> = Mutex::new(None);
// }

// macro_rules! get_db {
//     () => {
//         DB.lock()
//             .unwrap()
//             .as_ref()
//             .expect("connection not initialized yet!")
//     };
// }
