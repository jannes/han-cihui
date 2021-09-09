use anyhow::{anyhow, Result};
use std::{
    collections::{HashMap, HashSet},
    sync::{
        mpsc::{self, Receiver},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use rusqlite::Connection;

use crate::{
    analysis::{get_analysis_info, AnalysisInfo, AnalysisQuery},
    ebook::{open_as_flat_book, FlatBook},
    extraction::{extract_vocab, ExtractionResult},
    persistence::{select_known, sync_anki_data},
    segmentation::SegmentationMode,
    vocabulary::{get_known_chars, get_vocab_stats, VocabularyInfo},
};

pub struct State {
    pub analysis_state: AnalysisState,
    pub info_state: InfoState,
    pub current_view: View,
    pub db_connection: Arc<Mutex<Connection>>,
    pub action_log: Vec<String>,
}

impl State {
    pub fn new(db_connection: Connection) -> Result<Self> {
        let db_connection = Arc::new(Mutex::new(db_connection));
        Ok(State {
            analysis_state: AnalysisState::default(),
            info_state: InfoState::init(db_connection.clone())?,
            current_view: View::Info,
            db_connection,
            action_log: vec![],
        })
    }

    /// Is the user currently entering something in an input box?
    pub fn currently_input(&self) -> bool {
        match self.current_view {
            View::Analysis => matches!(
                self.analysis_state,
                AnalysisState::Opening(_, _) | AnalysisState::ExtractedSaving(_)
            ),
            _ => false,
        }
    }
}

pub enum View {
    Analysis,
    Info,
    Exit,
}

pub enum AnalysisState {
    Blank,
    // String arg: partial file path
    Opening(String, SegmentationMode),
    ExtractError(anyhow::Error),
    Extracting(ExtractingState),
    Extracted(ExtractedState),
    ExtractedSaving(ExtractedSavingState),
}

impl Default for AnalysisState {
    fn default() -> Self {
        AnalysisState::Blank
    }
}

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

pub struct ExtractQuery {
    pub filename: String,
    pub segmentation_mode: SegmentationMode,
}

pub struct ExtractingState {
    pub query: ExtractQuery,
    pub receiver: Receiver<Result<ExtractedState>>,
    pub extractor_thread: JoinHandle<()>,
    pub start: Instant,
}

fn extract(
    db_connection: Arc<Mutex<Connection>>,
    filename: &str,
    seg_mode: SegmentationMode,
) -> Result<ExtractedState> {
    let known_words: HashSet<String> = select_known(&db_connection.lock().unwrap())?
        .into_iter()
        .collect();
    let book = open_as_flat_book(filename, 1)?;
    let ext_res = extract_vocab(&book, seg_mode);
    Ok(ExtractedState::new(book, ext_res, known_words, true))
}

impl ExtractingState {
    pub fn new(query: ExtractQuery, db_connection: Arc<Mutex<Connection>>) -> Self {
        let (tx, rx) = mpsc::channel();
        let filename = query.filename.clone();
        let segmentation_mode = query.segmentation_mode;
        let extractor_thread = thread::spawn(move || {
            let res = extract(db_connection, &filename, segmentation_mode);
            tx.send(res).expect("could not send event");
        });
        ExtractingState {
            query,
            receiver: rx,
            extractor_thread,
            start: Instant::now(),
        }
    }

    // TODO: improve error handling
    // update state, return new state if extraction thread terminated, otherwise return None
    pub fn update(&mut self) -> Option<AnalysisState> {
        match self.receiver.try_recv() {
            Ok(res) => match res {
                Ok(extracted_state) => Some(AnalysisState::Extracted(extracted_state)),
                Err(e) => Some(AnalysisState::ExtractError(e)),
            },
            Err(e) => match e {
                mpsc::TryRecvError::Empty => None,
                mpsc::TryRecvError::Disconnected => Some(AnalysisState::ExtractError(anyhow!(
                    "Segmentation manager thread disconnected"
                ))),
            },
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}

pub struct ExtractedState {
    pub book: FlatBook,
    pub extraction_result: ExtractionResult,
    pub analysis_query: AnalysisQuery,
    pub analysis_infos: HashMap<AnalysisQuery, AnalysisInfo>,
    pub known_chars_are_known_words: bool,
    known_words: HashSet<String>,
    known_words_chars: HashSet<String>,
}

pub struct ExtractedSavingState {
    pub extracted_state: ExtractedState,
    pub partial_save_path: String,
}

impl ExtractedState {
    pub fn new(
        book: FlatBook,
        extraction_result: ExtractionResult,
        known_words: HashSet<String>,
        known_chars_are_known_words: bool,
    ) -> Self {
        let known_words_chars = known_words
            .union(&get_known_chars(&known_words))
            .map(|s| s.to_string())
            .collect::<HashSet<String>>();
        let known = if known_chars_are_known_words {
            &known_words_chars
        } else {
            &known_words
        };

        let query_all = AnalysisQuery {
            min_occurrence_words: 1,
            min_occurrence_unknown_chars: None,
        };
        let query_min3 = AnalysisQuery {
            min_occurrence_words: 3,
            min_occurrence_unknown_chars: None,
        };

        let mut analysis_infos = HashMap::new();
        let info_all = get_analysis_info(
            &extraction_result,
            query_all.min_occurrence_words,
            known,
            query_all.min_occurrence_unknown_chars,
        );
        let info_min3 = get_analysis_info(
            &extraction_result,
            query_min3.min_occurrence_words,
            known,
            query_min3.min_occurrence_unknown_chars,
        );
        analysis_infos.insert(query_all, info_all);
        analysis_infos.insert(query_min3, info_min3);

        ExtractedState {
            book,
            extraction_result,
            analysis_query: query_min3,
            analysis_infos,
            known_chars_are_known_words,
            known_words,
            known_words_chars,
        }
    }

    pub fn get_known_words(&self) -> &HashSet<String> {
        if self.known_chars_are_known_words {
            &self.known_words_chars
        } else {
            &self.known_words
        }
    }

    pub fn query_update(&mut self, query: AnalysisQuery) -> AnalysisInfo {
        let info = self.query(query);
        if self.analysis_infos.get(&query).is_none() {
            self.analysis_infos.insert(query, info);
        }
        self.analysis_query = query;
        info
    }

    pub fn query(&self, query: AnalysisQuery) -> AnalysisInfo {
        if let Some(info) = self.analysis_infos.get(&query) {
            *info
        } else {
            let known_words = if self.known_chars_are_known_words {
                &self.known_words_chars
            } else {
                &self.known_words
            };
            get_analysis_info(
                &self.extraction_result,
                query.min_occurrence_words,
                known_words,
                query.min_occurrence_unknown_chars,
            )
        }
    }

    pub fn query_all(&self) -> AnalysisInfo {
        self.query(AnalysisQuery {
            min_occurrence_words: 1,
            min_occurrence_unknown_chars: None,
        })
    }

    pub fn query_current(&self) -> AnalysisInfo {
        *self
            .analysis_infos
            .get(&self.analysis_query)
            .expect("current query should always be memoized already")
    }
}
