use crate::tui::state::info::SyncingState;
use crate::{tui::draw::util::get_centered_rect, vocabulary::VocabularyInfo};

use std::io::Write;
use tui::layout::{Alignment, Constraint, Direction, Layout};
use tui::text::Spans;
use tui::{
    backend::CrosstermBackend,
    layout::Rect,
    widgets::{Clear, Wrap},
};

use tui::widgets::{Block, Borders, Paragraph};
use tui::{
    style::{Color, Style},
    Frame,
};

pub fn draw_info(
    frame: &mut Frame<CrosstermBackend<impl Write>>,
    vocab_info: &VocabularyInfo,
    area: Rect,
) {
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .margin(2)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref());
    let chunks = layout.split(area);
    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White))
        .title("My vocabulary");
    let words_text = vocab_info
        .words_description()
        .into_iter()
        .map(Spans::from)
        .collect::<Vec<Spans>>();
    let chars_text = vocab_info
        .chars_description()
        .into_iter()
        .map(Spans::from)
        .collect::<Vec<Spans>>();
    let words_paragraph = Paragraph::new(words_text)
        .block(Block::default().title("词").borders(Borders::ALL))
        .alignment(Alignment::Right)
        .wrap(Wrap { trim: true });
    let chars_paragraph = Paragraph::new(chars_text)
        .block(Block::default().title("字").borders(Borders::ALL))
        .alignment(Alignment::Right)
        .wrap(Wrap { trim: true });
    frame.render_widget(block, area);
    frame.render_widget(words_paragraph, chunks[0]);
    frame.render_widget(chars_paragraph, chunks[1]);
}

pub fn draw_info_syncing(
    frame: &mut Frame<CrosstermBackend<impl Write>>,
    state: &SyncingState,
    area: Rect,
) {
    let amount_dots = (state.elapsed().as_secs() % 10) as usize;
    let text = format!("Syncing {}", ".".repeat(amount_dots));
    let area = get_centered_rect(area);
    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .title("Syncing vocabulary with Anki")
                .borders(Borders::ALL),
        )
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });
    frame.render_widget(Clear, area); //this clears out the background
    frame.render_widget(paragraph, area)
}
