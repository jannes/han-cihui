use crate::{
    analysis::{get_analysis_info, AnalysisInfo},
    extraction::ExtractionResult,
};
use anyhow::{Context, Result};
use crossterm::event;
use crossterm::event::KeyCode;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use std::{io, thread};
use tui::widgets::{Block, BorderType, Borders, Paragraph};
use tui::Terminal;
use tui::{
    backend::Backend,
    style::{Color, Style},
    Frame,
};
use tui::{backend::CrosstermBackend, layout::Rect};
use tui::{
    layout::{Alignment, Constraint, Direction, Layout},
    widgets::{Row, Table},
};

enum Event<I> {
    Input(I),
    Tick,
}

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
struct AnalysisQuery {
    min_occurrence_words: u64,
    min_occurrence_unknown_chars: Option<u64>,
}

pub fn enter_analysis_tui(
    extraction_result: &ExtractionResult,
    known_words: HashSet<String>,
) -> Result<()> {
    enable_raw_mode().expect("can run in raw mode");
    let (tx, rx) = mpsc::channel();
    let tick_rate = Duration::from_millis(200);
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout).expect("event poll does not work") {
                if let event::Event::Key(key) =
                    event::read().expect("could not read crossterm event")
                {
                    tx.send(Event::Input(key)).expect("could not send event");
                }
            }

            if last_tick.elapsed() >= tick_rate {
                if let Ok(_) = tx.send(Event::Tick) {
                    last_tick = Instant::now();
                }
            }
        }
    });

    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // STATE
    let mut analysis_infos: HashMap<AnalysisQuery, AnalysisInfo> = HashMap::new();
    let mut analysis_query = AnalysisQuery {
        min_occurrence_words: 3,
        min_occurrence_unknown_chars: None,
    };

    // QUERY and MEMOIZE STATE
    let mut get_query_analysis = |analysis_query: AnalysisQuery| {
        if let Some(info) = analysis_infos.get(&analysis_query) {
            *info
        } else {
            let info = get_analysis_info(
                extraction_result,
                analysis_query.min_occurrence_words,
                &known_words,
                analysis_query.min_occurrence_unknown_chars,
            );
            analysis_infos.insert(analysis_query, info);
            *analysis_infos.get(&analysis_query).unwrap()
        }
    };

    loop {
        terminal
            .draw(|rect| {
                let size = rect.size();
                let chunks = Layout::default()
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
                    .split(size);
                let header = get_header();
                let footer = get_footer(&analysis_query);
                let info_all = get_query_analysis(AnalysisQuery {
                    min_occurrence_words: 1,
                    min_occurrence_unknown_chars: None,
                });
                let info_min_occ = get_query_analysis(analysis_query);
                rect.render_widget(header, chunks[0]);
                render_analysis_info(&info_all, &info_min_occ, &analysis_query, rect, chunks[1]);
                rect.render_widget(footer, chunks[2]);
            })
            .context("error when drawing terminal")?;

        match rx.recv()? {
            Event::Input(event) => match event.code {
                KeyCode::Char('q') => {
                    disable_raw_mode()?;
                    terminal.show_cursor()?;
                    break;
                }
                // reduce min_occurrence of words
                KeyCode::Char('j') => {
                    analysis_query.min_occurrence_words = *analysis_query
                        .min_occurrence_words
                        .checked_sub(1)
                        .get_or_insert(0);
                }
                // increase min_occurrence of words
                KeyCode::Char('k') => {
                    analysis_query.min_occurrence_words += 1;
                }
                // reduce min_occurrence of unknown chars
                KeyCode::Char('h') => {
                    analysis_query.min_occurrence_unknown_chars =
                        match analysis_query.min_occurrence_unknown_chars {
                            Some(amount) => {
                                if amount == 1 {
                                    None
                                } else {
                                    Some(amount.checked_sub(1).unwrap())
                                }
                            }
                            None => None
                        };
                }
                // increase min_occurrence of unknown chars
                KeyCode::Char('l') => {
                    analysis_query.min_occurrence_unknown_chars =
                        match analysis_query.min_occurrence_unknown_chars {
                            Some(amount) => Some(amount + 1),
                            None => Some(1),
                        }
                }
                _ => {}
            },
            Event::Tick => {}
        }
    }

    Ok(())
}

fn render_analysis_info<B: Backend>(
    info_all: &AnalysisInfo,
    info_min_occ: &AnalysisInfo,
    min_occ_query: &AnalysisQuery,
    frame: &mut Frame<B>,
    area: Rect,
) -> Layout {
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .margin(2)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref());
    let chunks = layout.split(area);
    let block = Block::default().borders(Borders::ALL).title("Graphs");
    frame.render_widget(block, area);
    let all_chunk = chunks[0];
    let min_occ_chunk = chunks[1];
    frame.render_widget(
        get_analysis_info_table(info_all, "all words".to_string()),
        all_chunk,
    );
    let min_occ_title = match min_occ_query.min_occurrence_unknown_chars {
        Some(amount) => format!(
            "words occuring >= {}, or containing unknown chars occurring >= {}",
            min_occ_query.min_occurrence_words, amount
        ),
        None => format!("words occurring >= {}", min_occ_query.min_occurrence_words),
    };
    frame.render_widget(
        get_analysis_info_table(info_min_occ, min_occ_title),
        min_occ_chunk,
    );
    layout
}

fn get_analysis_info_table(info: &AnalysisInfo, title: String) -> Table {
    let row1 = Row::new(vec![
        "total words".to_string(),
        info.total_words.to_string(),
        info.unknown_total_words.to_string(),
    ]);
    let row2 = Row::new(vec![
        "unique words".to_string(),
        info.unique_words.to_string(),
        info.unknown_unique_words.to_string(),
    ]);
    let row3 = Row::new(vec![
        "total chars".to_string(),
        info.total_chars.to_string(),
        info.unknown_total_chars.to_string(),
    ]);
    let row4 = Row::new(vec![
        "unique chars".to_string(),
        info.unique_chars.to_string(),
        info.unknown_unique_chars.to_string(),
    ]);
    Table::new(vec![row1, row2, row3, row4])
        .header(
            Row::new(vec!["", "All", "Unknown"])
                .style(Style::default().fg(Color::Yellow))
                .bottom_margin(1),
        )
        .block(Block::default().borders(Borders::ALL).title(title))
        .widths(&[
            Constraint::Percentage(40),
            Constraint::Percentage(30),
            Constraint::Percentage(30),
        ])
}

fn get_header() -> Paragraph<'static> {
    Paragraph::new("Analyzing epub")
        .style(Style::default().fg(Color::LightCyan))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White))
                .border_type(BorderType::Plain),
        )
}

fn get_footer(current_analysis_query: &AnalysisQuery) -> Paragraph {
    let min_occ_unk_char = match current_analysis_query.min_occurrence_unknown_chars {
        Some(i) => format!("{}", i),
        None => "None".to_string(),
    };
    Paragraph::new(format!(
        "Query --- min words: {}, min unknown chars: {}",
        current_analysis_query.min_occurrence_words, min_occ_unk_char
    ))
    .style(Style::default().fg(Color::LightCyan))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White))
            .border_type(BorderType::Plain),
    )
}
