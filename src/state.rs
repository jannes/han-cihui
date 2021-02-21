use std::collections::{HashMap, HashSet};

use crate::{
    analysis::{get_analysis_info, AnalysisInfo, AnalysisQuery},
    ebook::Book,
    extraction::ExtractionResult,
    segmentation::SegmentationMode,
};

pub struct State {
    pub analysis_state: AnalysisState,
    pub info_state: InfoState,
    pub current_view: View,
}

impl Default for State {
    fn default() -> Self {
        State {
            analysis_state: AnalysisState::default(),
            info_state: InfoState::default(),
            current_view: View::Info,
        }
    }
}

impl State {
    fn get_tab_titles() -> Vec<String> {
        vec!["Info".to_string(), "Analysis".to_string()]
    }
}

pub enum View {
    Analysis,
    Info,
    Exit,
}

pub enum AnalysisState {
    Blank,
    Extract(ExtractQuery),
    Extracting,
    Extracted(ExtractedState),
}

impl Default for AnalysisState {
    fn default() -> Self {
        AnalysisState::Blank
    }
}

pub enum InfoState {
    Info,
    Syncing,
}

impl Default for InfoState {
    fn default() -> Self {
        InfoState::Info
    }
}

pub struct ExtractQuery {
    pub filename: String,
    pub segmentation_mode: SegmentationMode,
}

pub struct ExtractedState {
    pub book: Book,
    pub extraction_result: ExtractionResult,
    pub known_words: HashSet<String>,
    pub analysis_query: AnalysisQuery,
    pub analysis_infos: HashMap<AnalysisQuery, AnalysisInfo>,
}

impl ExtractedState {
    pub fn query_update(&mut self, query: AnalysisQuery) -> AnalysisInfo {
        let info = self.query(query);
        if self.analysis_infos.get(&query).is_none() {
            self.analysis_infos.insert(query, info);
        }
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
        *self.analysis_infos
            .get(&self.analysis_query)
            .expect("current query should always be memoized already")
    }
}
