use std::io::Write;

use tui::{backend::CrosstermBackend, layout::Rect, Frame};

use crate::tui::state::books;

pub fn draw_books_loading(frame: &mut Frame<CrosstermBackend<impl Write>>, area: Rect) {
    todo!()
}

pub fn draw_books_importing(
    frame: &mut Frame<CrosstermBackend<impl Write>>,
    partial_path: &str,
    area: Rect,
) {
    todo!()
}

pub fn draw_books_display(
    frame: &mut Frame<CrosstermBackend<impl Write>>,
    books: &books::DisplayState,
    area: Rect,
) {
    todo!()
}
