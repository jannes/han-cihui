use crate::{
    state::{
        AnalysisState, ExtractedSavingState, ExtractedState, ExtractingState, InfoState, State,
        SyncingState, View,
    },
    tui::util::{
        get_analysis_info_percentage_table, get_analysis_info_table, get_centered_rect,
        get_wrapping_spans, split_to_lines,
    },
    vocabulary::VocabularyInfo,
};
use anyhow::{Context, Error, Result};
use std::io::Write;
use tui::layout::{Alignment, Constraint, Direction, Layout, Margin};
use tui::{
    backend::CrosstermBackend,
    layout::Rect,
    widgets::{Clear, Wrap},
};
use tui::{style::Modifier, text::Spans, Terminal};
use tui::{
    style::{Color, Style},
    Frame,
};
use tui::{
    text::Span,
    widgets::{Block, BorderType, Borders, Paragraph, Tabs},
};

pub(super) fn draw_tab(
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
            AnalysisState::ExtractedSaving(saving_state) => {
                draw_analysis_saving(frame, saving_state, area);
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
        View::Exit => {}
    }
}

fn draw_info(
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

fn draw_info_syncing(
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

fn draw_analysis_extracted(
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

fn draw_analysis_extracting(
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

fn draw_analysis_extracted_error(
    frame: &mut Frame<CrosstermBackend<impl Write>>,
    error: &Error,
    area: Rect,
) {
    let msg = format!("{}\n\n\nPress [E] to open another file", error);
    draw_centered_input(frame, area, &msg, "Error extracting");
}

fn draw_analysis_saving(
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

fn draw_analysis_opening(
    frame: &mut Frame<CrosstermBackend<impl Write>>,
    partial_path: &str,
    area: Rect,
) {
    draw_centered_input(frame, area, partial_path, "Path to epub file to open")
}

fn draw_analysis_blank(frame: &mut Frame<CrosstermBackend<impl Write>>, area: Rect) {
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

fn draw_centered_input(
    frame: &mut Frame<CrosstermBackend<impl Write>>,
    area: Rect,
    partial_input: &str,
    box_title: &str,
) {
    let area = get_centered_rect(area);
    let input = get_wrapping_spans(partial_input, &area, None);
    let input_panel = Paragraph::new(input)
        .block(Block::default().borders(Borders::ALL).title(Span::styled(
            box_title,
            Style::default().add_modifier(Modifier::BOLD),
        )))
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Left);

    frame.render_widget(input_panel, area);
}

fn draw_header(frame: &mut Frame<CrosstermBackend<impl Write>>, state: &State, area: Rect) {
    let tab_titles = vec!["Info [0]".to_string(), "Analysis [1]".to_string()]
        .into_iter()
        .map(|s| Spans::from(Span::styled(s, Style::default().fg(Color::Yellow))))
        .collect();
    let selected = match state.current_view {
        View::Analysis => 1,
        View::Info => 0,
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
