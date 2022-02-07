use std::io::Write;

use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

use crate::tui::state::word_list::{ListOfWordLists, OpenedWordList};

pub fn draw_word_lists(
    frame: &mut Frame<CrosstermBackend<impl Write>>,
    state: &ListOfWordLists,
    area: Rect,
) {
    let header_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let header_cells = ["Book", "Author", "#w", "#c"]
        .iter()
        .map(|h| Cell::from(*h).style(header_style));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let selected_style = Style::default().add_modifier(Modifier::REVERSED);
    let rows = state.word_lists.iter().map(|wl| {
        let cells = vec![
            Cell::from(wl.book_name.clone()),
            Cell::from(wl.author_name.clone()),
            Cell::from(format!("{}", wl.analysis_query.min_occurrence_words)),
            Cell::from(format!(
                "{:?}",
                wl.analysis_query.min_occurrence_unknown_chars
            )),
        ];
        Row::new(cells)
    });
    let table = Table::new(rows)
        .header(header)
        .block(Block::default().borders(Borders::ALL))
        .highlight_style(selected_style)
        .highlight_symbol(">> ")
        .widths(&[
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ]);
    frame.render_stateful_widget(table, area, &mut state.table_state.borrow_mut());
}

pub fn draw_opened_word_list(
    frame: &mut Frame<CrosstermBackend<impl Write>>,
    state: &OpenedWordList,
    area: Rect,
) {
    let header_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let header_cells = ["Chapter", "Filtered", "To Learn", "Total"]
        .iter()
        .map(|h| Cell::from(*h).style(header_style));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let selected_style = Style::default().add_modifier(Modifier::REVERSED);
    let rows = state.chapter_infos().iter().map(|ci| {
        let cells = vec![
            Cell::from(ci.chapter_title().to_string()),
            Cell::from(ci.is_filtered().to_string()),
            Cell::from(ci.words_to_learn().to_string()),
            Cell::from(ci.words_total().to_string()),
        ];
        Row::new(cells)
    });
    let table = Table::new(rows)
        .header(header)
        .block(Block::default().borders(Borders::ALL))
        .highlight_style(selected_style)
        .highlight_symbol(">> ")
        .widths(&[
            Constraint::Percentage(40),
            Constraint::Percentage(10),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ]);
    frame.render_stateful_widget(table, area, &mut state.table_state.borrow_mut());
}
