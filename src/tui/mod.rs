mod draw;
mod events;

use crate::{
    analysis::AnalysisInfo,
    state::{State, View},
};
use anyhow::Result;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use crossterm::ExecutableCommand;
use crossterm::{event, terminal};
use std::time::{Duration, Instant};
use std::{io, thread};
use std::{io::Stdout, sync::mpsc};
use tui::backend::CrosstermBackend;
use tui::Terminal;
use tui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Row, Table},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use self::{
    draw::draw_tab,
    events::{handle_event, Event},
};

pub struct TuiApp {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    // enables taking out state, passing to handle_event, and putting result back
    // should always be Some(_), except right before & after handle_event call
    // TODO: find more elegant, type-safe way to handle this
    state: Option<State>,
}

impl TuiApp {
    pub fn new_stdout(state: State) -> Result<Self> {
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        let state = Some(state);
        Ok(Self { terminal, state })
    }

    pub fn run(mut self) -> Result<()> {
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
        self.terminal.clear()?;
        // 1. draw ui 2. listen for events
        // stop when state changes to exiting
        loop {
            draw_tab(self.state.as_ref().unwrap(), &mut self.terminal)?;
            let event = rx.recv()?;
            self.state = Some(handle_event(self.state.take().unwrap(), event)?);
            if let View::Exit = self.state.as_ref().unwrap().current_view {
                break;
            }
        }
        disable_raw_mode()?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}

impl Drop for TuiApp {
    fn drop(&mut self) {
        self.terminal
            .backend_mut()
            .execute(terminal::LeaveAlternateScreen)
            .expect("Could not execute to stdout");
        terminal::disable_raw_mode().expect("Terminal doesn't support to disable raw mode");
        if std::thread::panicking() {
            eprintln!("exit because of panic, to log the error redirect stderr to a file");
        }
    }
}

// ------- UTIL FUNCTIONS ---------

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

fn get_analysis_info_percentage_table<'a, 'b, 'c>(info: &'a AnalysisInfo, mic_occ_info: &'b AnalysisInfo) -> Table<'c> {
    let unknown_words_total_after = (info.unknown_total_words - mic_occ_info.unknown_total_words) as f64 / info.total_words as f64;
    let unknown_chars_total_after = (info.unknown_total_chars - mic_occ_info.unknown_total_chars) as f64 / info.total_chars as f64;
    let unknown_words_unique_after = (info.unknown_unique_words - mic_occ_info.unknown_unique_words) as f64 / info.unique_words as f64;
    let unknown_chars_unique_after = (info.unknown_unique_chars - mic_occ_info.unknown_unique_chars) as f64 / info.unique_chars as f64;
    let unknown_words_total_before = info.unknown_total_words as f64 / info.total_words as f64;
    let unknown_chars_total_before = info.unknown_total_chars as f64 / info.total_chars as f64;
    let unknown_words_unique_before = info.unknown_unique_words as f64 / info.unique_words as f64;
    let unknown_chars_unique_before = info.unknown_unique_chars as f64 / info.unique_chars as f64;
    let row1 = Row::new(vec![
        "total words".to_string(),
        format!("{:.3}", (1.0 - unknown_words_total_before)),
        format!("{:.3}", (1.0 - unknown_words_total_after)),
    ]);
    let row2 = Row::new(vec![
        "unique words".to_string(),
        format!("{:.3}", (1.0 - unknown_words_unique_before)),
        format!("{:.3}", (1.0 - unknown_words_unique_after)),
    ]);
    let row3 = Row::new(vec![
        "total chars".to_string(),
        format!("{:.3}", (1.0 - unknown_chars_total_before)),
        format!("{:.3}", (1.0 - unknown_chars_total_after)),
    ]);
    let row4 = Row::new(vec![
        "unique chars".to_string(),
        format!("{:.3}", (1.0 - unknown_chars_unique_before)),
        format!("{:.3}", (1.0 - unknown_chars_unique_after)),
    ]);
    Table::new(vec![row1, row2, row3, row4])
        .header(
            Row::new(vec!["", "Before", "After"])
                .style(Style::default().fg(Color::Yellow))
                .bottom_margin(1),
        )
        .block(Block::default().borders(Borders::ALL).title("Known before/after learning"))
        .widths(&[
            Constraint::Percentage(34),
            Constraint::Percentage(33),
            Constraint::Percentage(33),
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

// TODO: make this work with 汉字, split at grapheme boundaries
// COPIED FROM termchat
// split messages to fit the width of the UI panel
// to prevent overflow over the UI bounds
fn split_each(input: String, width: usize) -> Vec<String> {
    let mut splitted = Vec::with_capacity(input.width() / width);
    let mut row = String::new();

    let mut index = 0;

    for current_char in input.chars() {
        if (index != 0 && index == width)
            || current_char == '\n'
            || index + current_char.width().unwrap_or(0) > width
        {
            splitted.push(row.drain(..).collect());
            index = 0;
        }

        if current_char != '\n' {
            row.push(current_char);
        }
        index += current_char.width().unwrap_or(0);
    }
    // leftover
    if !row.is_empty() {
        splitted.push(row.drain(..).collect());
    }
    splitted
}
