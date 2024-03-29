use std::{
    cell::RefCell,
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result};
use rusqlite::Connection;
use tui::widgets::TableState;

use crate::{
    db::word_lists::{db_wlist_select_all_mdata, db_wlist_select_by_id},
    word_lists::{Category, ChapterWords, TaggedWord, WordList, WordListMetadata},
};

pub enum WordListState {
    List(ListOfWordLists),
    Opened(OpenedWordList),
}

impl WordListState {
    pub fn init(db: Arc<Mutex<Connection>>) -> Result<Self> {
        let word_lists =
            db_wlist_select_all_mdata(&db.lock().unwrap()).context("unable to load word lists")?;
        let table_state = RefCell::new(TableState::default());
        Ok(WordListState::List(ListOfWordLists {
            word_lists,
            table_state,
        }))
    }
}

pub struct ListOfWordLists {
    pub word_lists: Vec<WordListMetadata>,
    pub table_state: RefCell<TableState>,
}

impl ListOfWordLists {
    pub fn select_next(&mut self) {
        if self.word_lists.is_empty() {
            return;
        }
        let i = match self.table_state.borrow().selected() {
            Some(i) => {
                if i >= self.word_lists.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.borrow_mut().select(Some(i));
    }

    pub fn select_previous(&mut self) {
        if self.word_lists.is_empty() {
            return;
        }
        let i = match self.table_state.borrow().selected() {
            Some(i) => {
                if i == 0 {
                    self.word_lists.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.borrow_mut().select(Some(i));
    }

    pub fn try_open(self, db_conn: &Connection) -> Result<WordListState> {
        let wl_mdata = match self.table_state.borrow().selected() {
            Some(i) => self.word_lists.get(i),
            None => None,
        };
        match wl_mdata {
            Some(wl_mdata) => {
                let chapters = db_wlist_select_by_id(db_conn, wl_mdata.id)
                    .context("db error when selecting word list by id")?
                    .expect("word list with id did not exist in db");
                let wl = WordList {
                    metadata: wl_mdata.clone(),
                    words_per_chapter: chapters,
                };
                Ok(WordListState::Opened(OpenedWordList::new(wl)))
            }
            None => Ok(WordListState::List(self)),
        }
    }

    pub fn remove_current(&mut self) -> Option<WordListMetadata> {
        let (to_select, wlm) = match self.table_state.borrow().selected() {
            Some(i) => {
                let wlm = self.word_lists.remove(i);
                let index = if i >= self.word_lists.len() {
                    if i == 0 {
                        None
                    } else {
                        Some(i - 1)
                    }
                } else {
                    Some(i)
                };
                (index, Some(wlm))
            }
            None => (None, None),
        };
        self.table_state.borrow_mut().select(to_select);
        wlm
    }
}

pub struct WordListSummary {
    pub total: usize,
    pub filtered: usize,
    pub to_learn: usize,
    pub to_not_learn: usize,
    pub to_ignore: usize,
}

impl WordListSummary {
    pub fn new(chapter_infos: &[WLChapterInfo]) -> Self {
        let mut total = 0;
        let mut to_learn = 0;
        let mut to_not_learn = 0;
        let mut to_ignore = 0;

        for ch in chapter_infos {
            total += ch.words_total();
            to_learn += ch.words_to_learn();
            to_not_learn += ch.words_to_not_learn();
            to_ignore += ch.words_to_ignore();
        }
        let filtered = to_learn + to_not_learn + to_ignore;

        Self {
            total,
            filtered,
            to_learn,
            to_not_learn,
            to_ignore,
        }
    }
}

pub struct OpenedWordList {
    metadata: WordListMetadata,
    chapter_infos: Vec<WLChapterInfo>,
    summary: WordListSummary,
    pub table_state: RefCell<TableState>,
}

impl OpenedWordList {
    pub fn new(wl: WordList) -> Self {
        let metadata = wl.metadata;
        let chapter_infos: Vec<WLChapterInfo> = wl
            .words_per_chapter
            .into_iter()
            .map(WLChapterInfo::new)
            .collect();
        let summary = WordListSummary::new(&chapter_infos);
        Self {
            metadata,
            chapter_infos,
            summary,
            table_state: RefCell::new(TableState::default()),
        }
    }

    pub fn chapter_infos(&self) -> &Vec<WLChapterInfo> {
        &self.chapter_infos
    }

    pub fn summary(&self) -> &WordListSummary {
        &self.summary
    }

    pub fn get_selected_mut(&mut self) -> Option<(usize, &mut WLChapterInfo)> {
        if let Some(i) = self.table_state.borrow().selected() {
            return self.chapter_infos.get_mut(i).map(|ci| (i, ci));
        }
        None
    }

    pub fn sync(&mut self) {
        self.summary = WordListSummary::new(&self.chapter_infos);
    }

    pub fn get_selected(&self) -> Option<(usize, &WLChapterInfo)> {
        if let Some(i) = self.table_state.borrow().selected() {
            return self.chapter_infos.get(i).map(|ci| (i, ci));
        }
        None
    }

    pub fn select_next(&mut self) {
        if self.chapter_infos.is_empty() {
            return;
        }
        let i = match self.table_state.borrow().selected() {
            Some(i) => {
                if i >= self.chapter_infos.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.borrow_mut().select(Some(i));
    }

    pub fn select_previous(&mut self) {
        if self.chapter_infos.is_empty() {
            return;
        }
        let i = match self.table_state.borrow().selected() {
            Some(i) => {
                if i == 0 {
                    self.chapter_infos.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.borrow_mut().select(Some(i));
    }

    pub fn word_list_id(&self) -> i64 {
        self.metadata.id
    }

    pub fn word_list_metadata(&self) -> &WordListMetadata {
        &self.metadata
    }

    pub fn get_chapter_words(&self) -> Vec<&ChapterWords> {
        self.chapter_infos
            .iter()
            .map(|ci| &ci.chapter_words)
            .collect()
    }
}

pub struct WLChapterInfo {
    chapter_words: ChapterWords,
    words_to_learn: usize,
    words_to_not_learn: usize,
    words_to_ignore: usize,
    filtered: bool,
}

impl WLChapterInfo {
    pub fn new(cw: ChapterWords) -> Self {
        let mut res = Self {
            chapter_words: cw,
            words_to_learn: 0,
            words_to_not_learn: 0,
            words_to_ignore: 0,
            filtered: false,
        };
        res.update_status();
        res
    }

    pub fn chapter_title(&self) -> &str {
        &self.chapter_words.chapter_name
    }

    pub fn chapter_words(&self) -> &ChapterWords {
        &self.chapter_words
    }

    pub fn is_filtered(&self) -> bool {
        self.filtered
    }

    pub fn words_total(&self) -> usize {
        self.chapter_words.tagged_words.len()
    }

    pub fn words_to_learn(&self) -> usize {
        self.words_to_learn
    }

    pub fn words_to_not_learn(&self) -> usize {
        self.words_to_not_learn
    }

    pub fn words_to_ignore(&self) -> usize {
        self.words_to_ignore
    }

    pub fn get_words_to_learn(&self) -> Vec<&str> {
        self.chapter_words
            .tagged_words
            .iter()
            .filter_map(|tw| {
                if matches!(tw.category, Some(Category::Learn)) {
                    Some(tw.word.as_str())
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn modify_tw(&mut self, f: impl Fn(&mut Vec<TaggedWord>)) {
        f(&mut self.chapter_words.tagged_words);
        self.update_status();
    }

    fn update_status(&mut self) {
        self.filtered = true;
        self.words_to_learn = 0;
        self.chapter_words
            .tagged_words
            .iter()
            .for_each(|tw| match tw.category {
                Some(Category::Learn) => {
                    self.words_to_learn += 1;
                }
                Some(Category::NotLearn) => {
                    self.words_to_not_learn += 1;
                }
                Some(Category::Ignore) => {
                    self.words_to_ignore += 1;
                }
                None => {
                    self.filtered = false;
                }
            });
    }
}
