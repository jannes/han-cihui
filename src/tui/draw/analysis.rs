use crate::tui::draw::util::{
    get_analysis_info_percentage_table, get_analysis_info_table, get_centered_rect, split_to_lines,
};
use crate::tui::state::analysis::ExtractedState;
use std::io::Write;
use tui::layout::{Alignment, Constraint, Direction, Layout};
use tui::text::Spans;
use tui::{backend::CrosstermBackend, layout::Rect};
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

pub fn draw_analysis_blank(frame: &mut Frame<CrosstermBackend<impl Write>>, area: Rect) {
    let area = get_centered_rect(area);
    let inner_width = (area.width - 2) as usize;
    let msg = "go to books tab and select one for analysis";
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
