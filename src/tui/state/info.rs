use anyhow::Result;
use std::{
    sync::{
        mpsc::{self, Receiver},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use rusqlite::Connection;

use crate::{
    persistence::sync_anki_data,
    vocabulary::{get_vocab_stats, VocabularyInfo},
};

pub enum InfoState {
    Display(DisplayState),
    Syncing(SyncingState),
    // contains the previous vocabulary info
    SyncError(SyncErrorState),
}

impl InfoState {
    // getting vocab info is very fast, ok to block main thread
    pub fn init(db_connection: Arc<Mutex<Connection>>) -> Result<Self> {
        get_vocab_stats(&db_connection.lock().unwrap()).map(|vocab_info| {
            InfoState::Display(DisplayState {
                previous_vocab_info: None,
                vocab_info,
            })
        })
    }
}

pub struct DisplayState {
    pub previous_vocab_info: Option<VocabularyInfo>,
    pub vocab_info: VocabularyInfo,
}

impl DisplayState {
    // new - prev
    pub fn get_diff_active_words_chars(&self) -> Option<(i64, i64)> {
        self.previous_vocab_info.as_ref().map(|prev_vocab_info| {
            (
                self.vocab_info.words_active as i64 - prev_vocab_info.words_active as i64,
                self.vocab_info.chars_active_or_suspended_known as i64
                    - prev_vocab_info.chars_active_or_suspended_known as i64,
            )
        })
    }
}

pub struct SyncingState {
    pub previous_vocab_info: VocabularyInfo,
    pub receiver: Receiver<Result<VocabularyInfo>>,
    pub syncing_thread: JoinHandle<()>,
    pub start: Instant,
}

pub struct SyncErrorState {
    pub previous_vocab_info: VocabularyInfo,
    pub error_msg: String,
}

impl SyncingState {
    pub fn new(previous_vocab_info: VocabularyInfo, db_connection: Arc<Mutex<Connection>>) -> Self {
        let (tx, rx) = mpsc::channel();
        let syncing_thread = thread::spawn(move || {
            let db_conn = db_connection.lock().unwrap();
            let res = sync_anki_data(&db_conn).and_then(|()| get_vocab_stats(&db_conn));
            tx.send(res).expect("could not send event");
        });
        Self {
            previous_vocab_info,
            receiver: rx,
            syncing_thread,
            start: Instant::now(),
        }
    }

    // update state,
    // if syncing thread is done return: (new vocab info, diff to old vocab info) tuple
    pub fn update(&mut self) -> Option<InfoState> {
        match self.receiver.try_recv() {
            Ok(res) => match res {
                Ok(new_vocab_info) => Some(InfoState::Display(DisplayState {
                    previous_vocab_info: Some(self.previous_vocab_info),
                    vocab_info: new_vocab_info,
                })),
                Err(e) => Some(InfoState::SyncError(SyncErrorState {
                    previous_vocab_info: self.previous_vocab_info,
                    error_msg: e.to_string(),
                })),
            },
            Err(e) => match e {
                mpsc::TryRecvError::Empty => None,
                mpsc::TryRecvError::Disconnected => Some(InfoState::SyncError(SyncErrorState {
                    previous_vocab_info: self.previous_vocab_info,
                    error_msg: e.to_string(),
                })),
            },
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}
