use std::io::Write;

use tui::{
    backend::CrosstermBackend,
    layout::Rect,
    widgets::{List, ListItem},
    Frame,
};

use crate::word_lists::WordListMetadata;

pub fn draw_word_lists(
    frame: &mut Frame<CrosstermBackend<impl Write>>,
    area: Rect,
    word_lists: &Vec<WordListMetadata>,
) {
    let list_items: Vec<ListItem> = word_lists
        .iter()
        .map(|l| {
            ListItem::new(format!(
                "{} by {}, {}|{:?}",
                l.book_name,
                l.author_name,
                l.analysis_query.min_occurrence_words,
                l.analysis_query.min_occurrence_unknown_chars
            ))
        })
        .collect();
    let word_lists = List::new(list_items);
    frame.render_widget(word_lists, area);
}
