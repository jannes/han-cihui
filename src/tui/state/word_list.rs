use crate::word_lists::{WordList, WordListMetadata};

pub enum WordListState {
    ListOfWordLists { word_lists: Vec<WordListMetadata> },
    OpenedWordList { word_list: WordList },
}
