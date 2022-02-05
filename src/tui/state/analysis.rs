use crate::{
    analysis::{get_analysis_info, AnalysisInfo, AnalysisQuery},
    db::vocab::db_words_select_known,
    ebook::{open_as_flat_book, FlatBook},
    extraction::{extract_vocab, ExtractionResult},
    segmentation::SegmentationMode,
    vocabulary::get_known_chars,
};
use anyhow::{anyhow, Result};
use rusqlite::Connection;
use std::{
    collections::{HashMap, HashSet},
    sync::{
        mpsc::{self, Receiver},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

pub enum AnalysisState {
    Blank,
    // String arg: partial file path
    Opening(String, SegmentationMode),
    ExtractError(anyhow::Error),
    Extracting(ExtractingState),
    Extracted(ExtractedState),
}

impl Default for AnalysisState {
    fn default() -> Self {
        AnalysisState::Blank
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
    let known_words: HashSet<String> = db_words_select_known(&db_connection.lock().unwrap())?
        .into_iter()
        .collect();
    let book = open_as_flat_book(filename, 1)?;
    let ext_res = extract_vocab(&book, seg_mode);
    Ok(ExtractedState::new(book, ext_res, known_words, true))
}

impl ExtractingState {
    pub fn from_query(query: ExtractQuery, db_connection: Arc<Mutex<Connection>>) -> Self {
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
