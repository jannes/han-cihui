pub mod analysis;
pub mod info;
pub mod word_list;

use anyhow::Result;
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use self::{analysis::AnalysisState, info::InfoState};

pub struct State {
    pub analysis_state: AnalysisState,
    pub info_state: InfoState,
    // pub word_list_state: WordListState,
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
            View::Analysis => matches!(self.analysis_state, AnalysisState::Opening(_, _)),
            _ => false,
        }
    }
}

pub enum View {
    Analysis,
    Info,
    Exit,
}
