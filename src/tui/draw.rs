mod analysis;
mod info;
mod util;
mod word_list;

use anyhow::{Context, Result};
use std::io::Write;
use tui::layout::{Alignment, Constraint, Direction, Layout, Margin};
use tui::{backend::CrosstermBackend, layout::Rect};
use tui::{style::Modifier, text::Spans, Terminal};
use tui::{
    style::{Color, Style},
    Frame,
};
use tui::{
    text::Span,
    widgets::{Block, BorderType, Borders, Paragraph, Tabs},
};

use self::word_list::draw_word_lists;
use self::{
    analysis::{
        draw_analysis_blank, draw_analysis_extracted, draw_analysis_extracted_error,
        draw_analysis_extracting, draw_analysis_opening,
    },
    info::{draw_info, draw_info_syncing},
    util::get_wrapping_spans,
};

use super::state::analysis::AnalysisState;
use super::state::info::InfoState;
use super::state::word_list::WordListState;
use super::state::{State, View};

pub(super) fn draw_window(
    state: &State,
    terminal: &mut Terminal<CrosstermBackend<impl Write>>,
) -> Result<()> {
    terminal
        .draw(|rect| {
            let size = rect.size();
            let horizontal_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(80), Constraint::Percentage(20)].as_ref())
                .split(size);
            let main_chunk = horizontal_chunks[0];
            let action_log_margin = Margin {
                vertical: 2,
                horizontal: 0,
            };
            let action_log_chunk = horizontal_chunks[1].inner(&action_log_margin);
            let vertical_chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Min(2),
                        Constraint::Length(3),
                    ]
                    .as_ref(),
                )
                .split(main_chunk);
            draw_header(rect, state, vertical_chunks[0]);
            draw_inner(rect, state, vertical_chunks[1]);
            draw_footer(rect, state, vertical_chunks[2]);
            draw_action_log(rect, state, action_log_chunk);
        })
        .context("error when drawing terminal")?;
    Ok(())
}

fn draw_inner(frame: &mut Frame<CrosstermBackend<impl Write>>, state: &State, area: Rect) {
    match state.current_view {
        View::Analysis => match &state.analysis_state {
            AnalysisState::Blank => {
                draw_analysis_blank(frame, area);
            }
            AnalysisState::Opening(partial_path, _) => {
                draw_analysis_opening(frame, partial_path, area);
            }
            AnalysisState::Extracted(extracted_state) => {
                draw_analysis_extracted(frame, extracted_state, area);
            }
            AnalysisState::ExtractError(e) => {
                draw_analysis_extracted_error(frame, e, area);
            }
            AnalysisState::Extracting(extracting_state) => {
                draw_analysis_extracting(frame, extracting_state, area);
            }
        },
        View::Info => match &state.info_state {
            InfoState::Display(display_state) => draw_info(frame, &display_state.vocab_info, area),
            InfoState::Syncing(syncing_state) => draw_info_syncing(frame, syncing_state, area),
            // TODO: add visual indicator that sync error occurred
            InfoState::SyncError(sync_error_state) => {
                draw_info(frame, &sync_error_state.previous_vocab_info, area)
            }
        },
        View::WordLists => match &state.word_list_state {
            WordListState::ListOfWordLists {
                word_lists,
                selected,
            } => draw_word_lists(frame, area, word_lists, selected.clone()),
            WordListState::OpenedWordList { word_list } => todo!(),
        },
        View::Exit => {}
    }
}

fn draw_header(frame: &mut Frame<CrosstermBackend<impl Write>>, state: &State, area: Rect) {
    let tab_titles = vec![
        "Vocabulary [0]".to_string(),
        "Analysis [1]".to_string(),
        "Word Lists [2]".to_string(),
    ]
    .into_iter()
    .map(|s| Spans::from(Span::styled(s, Style::default().fg(Color::Yellow))))
    .collect();
    let selected = match state.current_view {
        View::Info => 0,
        View::Analysis => 1,
        View::WordLists => 2,
        View::Exit => 0,
    };
    let tabs = Tabs::new(tab_titles)
        .block(Block::default().borders(Borders::ALL))
        .select(selected)
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::Black),
        );
    frame.render_widget(tabs, area);
}

fn draw_footer(frame: &mut Frame<CrosstermBackend<impl Write>>, state: &State, area: Rect) {
    let text = match state.current_view {
        View::Analysis => {
            "[J]: - word occ | [K]: + word occ | [H]: - char occ | [L]: + char occ | [S]: save | [R]: reset"
        }
        View::Info => "[S]: sync Anki | [Q]: exit",
        View::WordLists => "[ESC]: overview | [Enter]: select | [J]: down | [K]: up",
        View::Exit => "EXITING",
    };
    let paragraph = Paragraph::new(text)
        .style(Style::default().fg(Color::LightGreen))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White))
                .border_type(BorderType::Plain),
        );
    frame.render_widget(paragraph, area);
}

fn draw_action_log(frame: &mut Frame<CrosstermBackend<impl Write>>, state: &State, area: Rect) {
    let action_msgs = state
        .action_log
        .iter()
        // latest msgs should be on top
        .rev()
        // split msgs into lines that fit in container
        .flat_map(|msg| get_wrapping_spans(msg, &area, Some("+ ")))
        .collect::<Vec<_>>();
    let action_log = Paragraph::new(action_msgs)
        .block(Block::default().borders(Borders::ALL).title("Action log"))
        .alignment(Alignment::Left);
    frame.render_widget(action_log, area)
}
