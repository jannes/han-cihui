use crate::{
    analysis::{get_analysis_info, AnalysisInfo, AnalysisQuery},
    extraction::ExtractionResult,
};
use std::collections::{HashMap, HashSet};

pub enum AnalysisState {
    Blank,
    Extracted(Box<ExtractedState>),
}

impl Default for AnalysisState {
    fn default() -> Self {
        AnalysisState::Blank
    }
}

pub struct ExtractedState {
    pub extraction_result: ExtractionResult,
    pub analysis_query: AnalysisQuery,
    pub analysis_infos: HashMap<AnalysisQuery, AnalysisInfo>,
    known_words_and_chars: HashSet<String>,
}

impl ExtractedState {
    pub fn new(
        extraction_result: ExtractionResult,
        known_words_and_chars: HashSet<String>,
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
            &known_words_and_chars,
            query_all.min_occurrence_unknown_chars,
        );
        let info_min3 = get_analysis_info(
            &extraction_result,
            query_min3.min_occurrence_words,
            &known_words_and_chars,
            query_min3.min_occurrence_unknown_chars,
        );
        analysis_infos.insert(query_all, info_all);
        analysis_infos.insert(query_min3, info_min3);

        ExtractedState {
            extraction_result,
            analysis_query: query_min3,
            analysis_infos,
            known_words_and_chars,
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
                &self.known_words_and_chars,
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
