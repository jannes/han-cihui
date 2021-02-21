use crate::{
    analysis::{
        AnalysisInfo,
    },
    state::{AnalysisState, ExtractedState, State, View},
};
use anyhow::{Context, Result};
use crossterm::event;
use crossterm::event::KeyCode;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use event::KeyEvent;
use std::{time::{Duration, Instant}, unimplemented};
use std::{io, thread};
use std::{io::Write, sync::mpsc};
use tui::{
    style::{Color, Style},
    Frame,
};
use tui::{backend::CrosstermBackend, layout::Rect};
use tui::{
    layout::{Alignment, Constraint, Direction, Layout},
    widgets::{Row, Table},
};
use tui::{style::Modifier, text::Spans, Terminal};
use tui::{
    text::Span,
    widgets::{Block, BorderType, Borders, Paragraph, Tabs},
};

enum Event<I> {
    Input(I),
    Tick,
}

pub fn enter_tui(mut state: State) -> Result<()> {
    enable_raw_mode().expect("can run in raw mode");
    let (tx, rx) = mpsc::channel();
    let tick_rate = Duration::from_millis(20);
    // listen to key events on background thread, which sends them through channel
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

            if last_tick.elapsed() >= tick_rate && tx.send(Event::Tick).is_ok() {
                last_tick = Instant::now();
            }
        }
    });
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    // 1. draw ui 2. listen for events
    // stop when state changes to exiting
    loop {
        draw_tab(&state, &mut terminal)?;
        let event = rx.recv()?;
        state = handle_event(state, event)?;
        if let View::Exit = state.current_view {
            break;
        }
    }
    disable_raw_mode()?;
    terminal.show_cursor()?;
    Ok(())
}

fn draw_tab(state: &State, terminal: &mut Terminal<CrosstermBackend<impl Write>>) -> Result<()> {
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
            draw_header(rect, &state, chunks[0]);
            draw_inner(rect, &state, chunks[1]);
            draw_footer(rect, &state, chunks[2]);
        })
        .context("error when drawing terminal")?;
    Ok(())
}

fn handle_event(mut state: State, event: Event<KeyEvent>) -> Result<State> {
    let key_event = match event {
        Event::Input(key_event) => match key_event.code {
            KeyCode::Char('q') => {
                state.current_view = View::Exit;
                return Ok(state);
            }
            KeyCode::Char('0') => {
                state.current_view = View::Info;
                return Ok(state);
            }
            KeyCode::Char('1') => {
                state.current_view = View::Analysis;
                return Ok(state);
            }
            _ => key_event,
        },
        Event::Tick => {
            return Ok(state);
        }
    };
    match state.current_view {
        View::Analysis => match &mut state.analysis_state {
            AnalysisState::Blank => {}
            AnalysisState::Extract(_) => {}
            AnalysisState::Extracting => {}
            AnalysisState::Extracted(extracted_state) => {
                handle_event_analysis_extracted(extracted_state, key_event)?;
            }
        },
        View::Info => {
            handle_event_info(&mut state, key_event)?;
        },
        View::Exit => {},
    };
    Ok(state)
}

fn handle_event_analysis_extracted(extracted_state: &mut ExtractedState, key_event: KeyEvent) -> Result<()> {
    let mut analysis_query = extracted_state.analysis_query;
    match key_event.code {
        KeyCode::Char('s') => {}
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
                    None => None,
                };
        }
        // increase min_occurrence of unknown chars
        KeyCode::Char('l') => {
            analysis_query.min_occurrence_unknown_chars =
                match extracted_state.analysis_query.min_occurrence_unknown_chars {
                    Some(amount) => Some(amount + 1),
                    None => Some(1),
                }
        }
        _ => {}
    }
    extracted_state.query_update(analysis_query);
    Ok(())
}

fn handle_event_info(state: &mut State, key_event: KeyEvent) -> Result<()> {
    unimplemented!()
}

fn draw_inner(frame: &mut Frame<CrosstermBackend<impl Write>>, state: &State, area: Rect) {
    match state.current_view {
        View::Analysis => match &state.analysis_state {
            AnalysisState::Blank => {}
            AnalysisState::Extract(_) => {}
            AnalysisState::Extracting => {}
            AnalysisState::Extracted(extracted_state) => {
                draw_analysis_extracted(frame, extracted_state, area);
            }
        },
        View::Info => draw_info(frame, state, area),
        View::Exit => {}
    }
}

fn draw_info(frame: &mut Frame<CrosstermBackend<impl Write>>, state: &State, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("My vocabulary (TODO)");
    frame.render_widget(block, area);
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
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref());
    let chunks = layout.split(area);
    let block = Block::default()
        .borders(Borders::ALL)
        .title("词/字 occurrence info");
    frame.render_widget(block, area);
    let all_chunk = chunks[0];
    let min_occ_chunk = chunks[1];
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
        .block(Block::default().borders(Borders::ALL).title("Tabs"))
        .select(selected)
        .style(Style::default().fg(Color::Cyan))
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
            "[J]: - word occ [K]: + word occ [H]: - char occ [L]: + char occ [S]: save"
        }
        View::Info => "[Q]: exit",
        View::Exit => "EXITING",
    };
    let paragraph =
        Paragraph::new(text)
            .style(Style::default().fg(Color::LightCyan))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(Style::default().fg(Color::White))
                    .border_type(BorderType::Plain),
            );
    frame.render_widget(paragraph, area);
}

// fn get_header() -> Paragraph<'static> {
//     Paragraph::new("Analyzing epub")
//         .style(Style::default().fg(Color::LightCyan))
//         .alignment(Alignment::Center)
//         .block(
//             Block::default()
//                 .borders(Borders::ALL)
//                 .style(Style::default().fg(Color::White))
//                 .border_type(BorderType::Plain),
//         )
// }

// fn get_footer() -> Paragraph<'static> {
//     Paragraph::new("[J]: - word occ [K]: + word occ [H]: - char occ [L]: + char occ [S]: save")
//         .style(Style::default().fg(Color::LightCyan))
//         .alignment(Alignment::Center)
//         .block(
//             Block::default()
//                 .borders(Borders::ALL)
//                 .style(Style::default().fg(Color::White))
//                 .border_type(BorderType::Plain),
//         )
// }

// pub fn enter_analysis_tui(
//     book: &Book,
//     extraction_result: &ExtractionResult,
//     known_words: HashSet<String>,
// ) -> Result<()> {
//     enable_raw_mode().expect("can run in raw mode");
//     let (tx, rx) = mpsc::channel();
//     let tick_rate = Duration::from_millis(200);
//     thread::spawn(move || {
//         let mut last_tick = Instant::now();
//         loop {
//             let timeout = tick_rate
//                 .checked_sub(last_tick.elapsed())
//                 .unwrap_or_else(|| Duration::from_secs(0));

//             if event::poll(timeout).expect("event poll does not work") {
//                 if let event::Event::Key(key) =
//                     event::read().expect("could not read crossterm event")
//                 {
//                     tx.send(Event::Input(key)).expect("could not send event");
//                 }
//             }

//             if last_tick.elapsed() >= tick_rate {
//                 if let Ok(_) = tx.send(Event::Tick) {
//                     last_tick = Instant::now();
//                 }
//             }
//         }
//     });

//     let stdout = io::stdout();
//     let backend = CrosstermBackend::new(stdout);
//     let mut terminal = Terminal::new(backend)?;
//     terminal.clear()?;

//     // STATE
//     let mut analysis_infos: HashMap<AnalysisQuery, AnalysisInfo> = HashMap::new();
//     let mut analysis_query = AnalysisQuery {
//         min_occurrence_words: 3,
//         min_occurrence_unknown_chars: None,
//     };
//     let mut save_result = false;

//     // QUERY and MEMOIZE STATE
//     let mut get_query_analysis = |analysis_query: AnalysisQuery| {
//         if let Some(info) = analysis_infos.get(&analysis_query) {
//             *info
//         } else {
//             let info = get_analysis_info(
//                 extraction_result,
//                 analysis_query.min_occurrence_words,
//                 &known_words,
//                 analysis_query.min_occurrence_unknown_chars,
//             );
//             analysis_infos.insert(analysis_query, info);
//             *analysis_infos.get(&analysis_query).unwrap()
//         }
//     };

//     loop {
//         terminal
//             .draw(|rect| {
//                 let size = rect.size();
//                 let chunks = Layout::default()
//                     .direction(Direction::Vertical)
//                     .margin(2)
//                     .constraints(
//                         [
//                             Constraint::Length(3),
//                             Constraint::Min(2),
//                             Constraint::Length(3),
//                         ]
//                         .as_ref(),
//                     )
//                     .split(size);
//                 let header = get_header();
//                 let footer = get_footer();
//                 let info_all = get_query_analysis(AnalysisQuery {
//                     min_occurrence_words: 1,
//                     min_occurrence_unknown_chars: None,
//                 });
//                 let info_min_occ = get_query_analysis(analysis_query);
//                 rect.render_widget(header, chunks[0]);
//                 render_analysis_info(&info_all, &info_min_occ, &analysis_query, rect, chunks[1]);
//                 rect.render_widget(footer, chunks[2]);
//             })
//             .context("error when drawing terminal")?;

//         match rx.recv()? {
//             Event::Input(event) => match event.code {
//                 KeyCode::Char('q') => {
//                     disable_raw_mode()?;
//                     terminal.show_cursor()?;
//                     break;
//                 }
//                 KeyCode::Char('s') => {
//                     disable_raw_mode()?;
//                     terminal.show_cursor()?;
//                     save_result = true;
//                     break;
//                 }
//                 // reduce min_occurrence of words
//                 KeyCode::Char('j') => {
//                     analysis_query.min_occurrence_words = *analysis_query
//                         .min_occurrence_words
//                         .checked_sub(1)
//                         .get_or_insert(0);
//                 }
//                 // increase min_occurrence of words
//                 KeyCode::Char('k') => {
//                     analysis_query.min_occurrence_words += 1;
//                 }
//                 // reduce min_occurrence of unknown chars
//                 KeyCode::Char('h') => {
//                     analysis_query.min_occurrence_unknown_chars =
//                         match analysis_query.min_occurrence_unknown_chars {
//                             Some(amount) => {
//                                 if amount == 1 {
//                                     None
//                                 } else {
//                                     Some(amount.checked_sub(1).unwrap())
//                                 }
//                             }
//                             None => None,
//                         };
//                 }
//                 // increase min_occurrence of unknown chars
//                 KeyCode::Char('l') => {
//                     analysis_query.min_occurrence_unknown_chars =
//                         match analysis_query.min_occurrence_unknown_chars {
//                             Some(amount) => Some(amount + 1),
//                             None => Some(1),
//                         }
//                 }
//                 _ => {}
//             },
//             Event::Tick => {}
//         }
//     }
//     if save_result {
//         println!("type path where result json should be saved, then press enter");
//         let stdin = io::stdin();
//         let path = stdin
//             .lock()
//             .lines()
//             .next()
//             .expect("expected one line of input")
//             .context("could not read line from stdin")?;
//         let filtered_extraction_set = get_filtered_extraction_items(
//             extraction_result,
//             analysis_query.min_occurrence_words,
//             &known_words,
//             analysis_query.min_occurrence_unknown_chars,
//         );
//         save_filtered_extraction_info(&book, &filtered_extraction_set, &path)?;
//         println!("saved result");
//     }
//     println!("exiting");
//     Ok(())
// }

// fn draw_analysis_info<B: Backend>(
//     info_all: &AnalysisInfo,
//     info_min_occ: &AnalysisInfo,
//     min_occ_query: &AnalysisQuery,
//     frame: &mut Frame<B>,
//     area: Rect,
// ) -> Layout {
//     let layout = Layout::default()
//         .direction(Direction::Horizontal)
//         .margin(2)
//         .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref());
//     let chunks = layout.split(area);
//     let block = Block::default()
//         .borders(Borders::ALL)
//         .title("词/字 occurrence info");
//     frame.render_widget(block, area);
//     let all_chunk = chunks[0];
//     let min_occ_chunk = chunks[1];
//     frame.render_widget(
//         get_analysis_info_table(info_all, "all words".to_string()),
//         all_chunk,
//     );
//     let min_occ_title = match min_occ_query.min_occurrence_unknown_chars {
//         Some(amount) => format!(
//             "#word >= {} OR contains unknown #char >= {}",
//             min_occ_query.min_occurrence_words, amount
//         ),
//         None => format!("#word >= {}", min_occ_query.min_occurrence_words),
//     };
//     frame.render_widget(
//         get_analysis_info_table(info_min_occ, min_occ_title),
//         min_occ_chunk,
//     );
//     layout
// }