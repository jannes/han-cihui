use std::io::prelude::*;
use std::os::unix::net::UnixStream;
use std::{cell::RefCell, rc::Rc};

use han_cihui::config::tagging_socket_path;
use han_cihui::word_lists::{Category, TaggedWord};
use slint::quit_event_loop;

slint::include_modules!();

pub fn main() {
    let mut stream = UnixStream::connect(tagging_socket_path()).expect("could not open stream");
    let mut n: [u8; 4] = [0; 4];
    stream.read_exact(&mut n).expect("could not read n");
    let n = u32::from_be_bytes(n) as usize;

    let mut words_serialized: Vec<u8> = vec![0; n];
    stream
        .read_exact(&mut words_serialized[0..n])
        .expect("could not read words");
    let mut words: Vec<TaggedWord> =
        serde_json::from_slice(&words_serialized).expect("could not deserialize");

    tag_words(&mut words);

    serde_json::to_writer(stream, &words).expect("could not write into stream");
}

pub fn tag_words(words: &mut Vec<TaggedWord>) {
    let state = State::new(words);
    let state = Rc::new(RefCell::new(state));
    // construct state and ui
    let ui = AppWindow::new();
    let current_word = state.borrow().current_word().unwrap();
    ui.set_current_word(current_word.into());
    ui.set_footer(get_footer_tagging().into());

    // closure for updating state & ui
    let ui_weak = ui.as_weak();
    let state_clone = state.clone();
    let apply_cmd = move |command: Command| {
        state_clone.borrow_mut().apply_command(command);
        let ui = ui_weak.unwrap();
        ui.set_current_word(
            state_clone
                .borrow()
                .current_word()
                .unwrap_or_else(|| "END".to_string())
                .into(),
        );
        if state_clone.borrow().current_word().is_none() {
            ui.set_footer(get_footer_finished().into());
        } else {
            ui.set_footer(get_footer_tagging().into());
        }
    };

    let ui_weak = ui.as_weak();
    // link key press to update closure, register close events
    ui.on_key_event(move |k| match k.as_str() {
        "j" => apply_cmd(Command::Tag(Category::Learn)),
        "k" => apply_cmd(Command::Tag(Category::NotLearn)),
        "l" => apply_cmd(Command::Tag(Category::Ignore)),
        "u" => apply_cmd(Command::Undo),
        // ESC
        "\u{1b}" => {
            let _ = quit_event_loop();
        }
        // Enter
        "\n" => {
            ui_weak.unwrap().hide();
            let _ = quit_event_loop();
        }
        _ => {}
    });

    // start ui, return final words when done
    ui.run();
    *words = state.borrow().get_words();
}

struct State {
    words: Vec<TaggedWord>,
    index: usize,
}

enum Command {
    Undo,
    Tag(Category),
}

impl State {
    fn new(words: &[TaggedWord]) -> Self {
        Self {
            words: words.to_owned(),
            index: 0,
        }
    }

    fn apply_command(&mut self, command: Command) {
        match command {
            Command::Undo => {
                if self.index > 0 {
                    self.index -= 1;
                    self.words.get_mut(self.index).unwrap().reset();
                }
            }
            Command::Tag(category) => {
                if self.index < self.words.len() {
                    self.words.get_mut(self.index).unwrap().tag(category);
                    self.index += 1;
                }
            }
        }
    }

    fn current_word(&self) -> Option<String> {
        self.words.get(self.index).map(|w| w.word.clone())
    }

    fn get_words(&self) -> Vec<TaggedWord> {
        self.words.clone()
    }
}

fn get_footer_tagging() -> &'static str {
    "[J]: learn | [K]: not learn | [L]: ignore | [U]: undo | ESC: exit"
}

fn get_footer_finished() -> &'static str {
    "[U]: undo | ENTER: confirm and exit"
}
