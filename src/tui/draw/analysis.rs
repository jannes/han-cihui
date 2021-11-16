use crate::tui::draw::util::{
    draw_centered_input, get_analysis_info_percentage_table, get_analysis_info_table,
    get_centered_rect, split_to_lines,
};
use crate::tui::state::analysis::{ExtractedSavingState, ExtractedState, ExtractingState};
use anyhow::Error;
use std::io::Write;
use tui::layout::{Alignment, Constraint, Direction, Layout};
use tui::text::Spans;
use tui::{
    backend::CrosstermBackend,
    layout::Rect,
    widgets::{Clear, Wrap},
};
use tui::{
    style::{Color, Style},
    Frame,
};
use tui::{
    text::Span,
    widgets::{Block, Borders, Paragraph},
};

pub fn draw_analysis_extracted(
    frame: &mut Frame<CrosstermBackend<impl Write>>,
    state: &ExtractedState,
    area: Rect,
) {
    let info_all = state.query_all();
    let info_min_occ = state.query_current();
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .margin(2)
        .constraints(
            [
                Constraint::Percentage(33),
                Constraint::Percentage(33),
                Constraint::Percentage(34),
            ]
            .as_ref(),
        );
    let chunks = layout.split(area);
    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White))
        .title("词/字 occurrence info");
    frame.render_widget(block, area);
    let all_chunk = chunks[0];
    let min_occ_chunk = chunks[1];
    let perc_chunk = chunks[2];
    frame.render_widget(
        get_analysis_info_table(&info_all, "all words".to_string()),
        all_chunk,
    );
    let min_occ_title = match state.analysis_query.min_occurrence_unknown_chars {
        Some(amount) => format!(
            "#word >= {} OR contains unknown #char >= {}",
            state.analysis_query.min_occurrence_words, amount
        ),
        None => format!("#word >= {}", state.analysis_query.min_occurrence_words),
    };
    frame.render_widget(
        get_analysis_info_table(&info_min_occ, min_occ_title),
        min_occ_chunk,
    );
    frame.render_widget(
        get_analysis_info_percentage_table(&info_all, &info_min_occ),
        perc_chunk,
    );
}

pub fn draw_analysis_extracting(
    frame: &mut Frame<CrosstermBackend<impl Write>>,
    state: &ExtractingState,
    area: Rect,
) {
    let amount_dots = (state.elapsed().as_secs() % 10) as usize;
    let text = format!("Extracting {}", ".".repeat(amount_dots));
    let area = get_centered_rect(area);
    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .title("Extracting from epub")
                .borders(Borders::ALL),
        )
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });
    frame.render_widget(Clear, area); //this clears out the background
    frame.render_widget(paragraph, area);
}

pub fn draw_analysis_extracted_error(
    frame: &mut Frame<CrosstermBackend<impl Write>>,
    error: &Error,
    area: Rect,
) {
    let msg = format!("{}\n\n\nPress [E] to open another file", error);
    draw_centered_input(frame, area, &msg, "Error extracting");
}

pub fn draw_analysis_saving(
    frame: &mut Frame<CrosstermBackend<impl Write>>,
    state: &ExtractedSavingState,
    area: Rect,
) {
    draw_centered_input(
        frame,
        area,
        &state.partial_save_path,
        "Path to save json result file to",
    )
}

pub fn draw_analysis_opening(
    frame: &mut Frame<CrosstermBackend<impl Write>>,
    partial_path: &str,
    area: Rect,
) {
    draw_centered_input(frame, area, partial_path, "Path to epub file to open")
}

pub fn draw_analysis_blank(frame: &mut Frame<CrosstermBackend<impl Write>>, area: Rect) {
    let area = get_centered_rect(area);
    let inner_width = (area.width - 2) as usize;
    let msg = "press [E] to enter path of epub to extract vocab from";
    let msg = split_to_lines(msg, inner_width, None)
        .into_iter()
        .map(|line| Spans::from(vec![Span::raw(line)]))
        .collect::<Vec<_>>();

    let msg_panel = Paragraph::new(msg)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Left);

    frame.render_widget(msg_panel, area);
}
