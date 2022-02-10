use std::{cell::RefCell, rc::Rc};

use slint::quit_event_loop;

use crate::word_lists::{Category, TaggedWord};

slint::include_modules!();

pub fn tag_words(words: &mut Vec<TaggedWord>) {
    // construct state and ui
    let state = Rc::new(RefCell::new(State::new(words)));
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

    // link key press to update closure, register close events
    ui.on_key_event(move |k| match k.as_str() {
        "j" => apply_cmd(Command::Tag(Category::Learn)),
        "k" => apply_cmd(Command::Tag(Category::NotLearn)),
        "l" => apply_cmd(Command::Tag(Category::Ignore)),
        "u" => apply_cmd(Command::Undo),
        "\u{1b}" => quit_event_loop(), // ESC
        "\n" => quit_event_loop(),     // Enter
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
    fn new(words: &Vec<TaggedWord>) -> Self {
        Self {
            words: words.clone(),
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
