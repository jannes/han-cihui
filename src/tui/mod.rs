mod draw;
mod events;
pub mod state;

use anyhow::Result;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use crossterm::ExecutableCommand;
use crossterm::{event, terminal};
use std::time::{Duration, Instant};
use std::{io, thread};
use std::{io::Stdout, sync::mpsc};
use tui::backend::CrosstermBackend;
use tui::Terminal;

use self::state::{State, View};
use self::{
    draw::draw_window,
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
            draw_window(self.state.as_ref().unwrap(), &mut self.terminal)?;
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
