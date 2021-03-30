use anyhow::Result;
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
    ebook::{open_as_book, Book},
    extraction::{extract_vocab, ExtractionResult},
    persistence::select_known,
    segmentation::SegmentationMode,
    vocabulary::{get_vocab_stats, VocabularyInfo},
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
            View::Analysis => matches!(self.analysis_state, AnalysisState::ExtractedSaving(_)),
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
    Opening(String, SegmentationMode),
    ExtractError,
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
    Info(VocabularyInfo),
    Syncing,
}

impl InfoState {
    // getting vocab info is very fast, ok to block main thread
    pub fn init(db_connection: Arc<Mutex<Connection>>) -> Result<Self> {
        get_vocab_stats(db_connection).map(InfoState::Info)
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
    let book = open_as_book(filename)?;
    let ext_res = extract_vocab(&book, seg_mode);
    Ok(ExtractedState::new(book, ext_res, known_words))
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
                Err(_e) => Some(AnalysisState::ExtractError),
            },
            Err(e) => match e {
                mpsc::TryRecvError::Empty => None,
                mpsc::TryRecvError::Disconnected => Some(AnalysisState::ExtractError),
            },
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}

pub struct ExtractedState {
    pub book: Book,
    pub extraction_result: ExtractionResult,
    pub known_words: HashSet<String>,
    pub analysis_query: AnalysisQuery,
    pub analysis_infos: HashMap<AnalysisQuery, AnalysisInfo>,
}

pub struct ExtractedSavingState {
    pub extracted_state: ExtractedState,
    pub partial_save_path: String,
}

impl ExtractedState {
    pub fn new(
        book: Book,
        extraction_result: ExtractionResult,
        known_words: HashSet<String>,
    ) -> Self {
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
            &known_words,
            query_all.min_occurrence_unknown_chars,
        );
        let info_min3 = get_analysis_info(
            &extraction_result,
            query_min3.min_occurrence_words,
            &known_words,
            query_min3.min_occurrence_unknown_chars,
        );
        analysis_infos.insert(query_all, info_all);
        analysis_infos.insert(query_min3, info_min3);
        ExtractedState {
            book,
            extraction_result,
            known_words,
            analysis_query: query_min3,
            analysis_infos,
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
            get_analysis_info(
                &self.extraction_result,
                query.min_occurrence_words,
                &self.known_words,
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
