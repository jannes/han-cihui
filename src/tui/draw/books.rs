use crate::tui::draw::util::get_centered_rect;
use std::io::Write;

use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap},
    Frame,
};

use crate::tui::state::books;

use super::util::draw_centered_input;

pub fn draw_books_loading(
    frame: &mut Frame<CrosstermBackend<impl Write>>,
    msg: &str,
    elapsed_secs: u64,
    area: Rect,
) {
    let amount_dots = (elapsed_secs % 10) as usize;
    let text = format!("{} {}", msg, ".".repeat(amount_dots));
    let area = get_centered_rect(area);
    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .title("Loading book view")
                .borders(Borders::ALL),
        )
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });
    frame.render_widget(Clear, area); //this clears out the background
    frame.render_widget(paragraph, area)
}

pub fn draw_books_display(
    frame: &mut Frame<CrosstermBackend<impl Write>>,
    state: &books::DisplayState,
    area: Rect,
) {
    let header_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let header_cells = ["Book", "Author", "Comprehension", "Length"]
        .iter()
        .map(|h| Cell::from(*h).style(header_style));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let selected_style = Style::default().add_modifier(Modifier::REVERSED);
    let rows = state.books_with_stats.iter().map(|b| {
        let cells = vec![
            Cell::from(b.title.clone()),
            Cell::from(b.author.clone()),
            Cell::from(format!("{}", b.word_comprehension)),
            Cell::from(format!("{}", b.total_chars)),
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

pub fn draw_books_importing(
    frame: &mut Frame<CrosstermBackend<impl Write>>,
    partial_path: &str,
    area: Rect,
) {
    draw_centered_input(frame, area, partial_path, "Path to epub file to import")
}
