mod draw;
mod events;

use crate::{analysis::AnalysisInfo, state::{State, View}};
use anyhow::Result;
use crossterm::event;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use std::{io, thread};
use tui::backend::CrosstermBackend;
use tui::Terminal;
use tui::{layout::{Constraint, Direction, Layout, Rect}, style::{Color, Style}, widgets::{Block, Borders, Row, Table}};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use self::{draw::draw_tab, events::{Event, handle_event}};

pub fn enter_tui(mut state: State) -> Result<()> {
    enable_raw_mode().expect("can run in raw mode");
    let (tx, rx) = mpsc::channel();
    let tick_rate = Duration::from_millis(200);
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

// UTIL FUNCTIONS

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

fn get_centered_rect(r: Rect) -> Rect {
    let percent_y = 50;
    let percent_x = 50;
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}

// COPIED FROM termchat
// TODO: check if really needed
// split messages to fit the width of the ui panel
fn split_each(input: String, width: usize) -> Vec<String> {
    let mut splitted = Vec::with_capacity(input.width() / width);
    let mut row = String::new();

    let mut index = 0;

    for current_char in input.chars() {
        if (index != 0 && index == width) || index + current_char.width().unwrap_or(0) > width {
            splitted.push(row.drain(..).collect());
            index = 0;
        }

        row.push(current_char);
        index += current_char.width().unwrap_or(0);
    }
    // leftover
    if !row.is_empty() {
        splitted.push(row.drain(..).collect());
    }
    splitted
}
