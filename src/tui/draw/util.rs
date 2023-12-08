use std::io::Write;

use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph, Row, Table},
    Frame,
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::analysis::AnalysisInfo;

// ------- UTIL FUNCTIONS - DATA/UI ---------

pub fn get_analysis_info_table(info: &AnalysisInfo, title: String) -> Table {
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

pub fn get_analysis_info_percentage_table<'c>(
    info: &AnalysisInfo,
    mic_occ_info: &AnalysisInfo,
) -> Table<'c> {
    let unknown_words_total_after = (info.unknown_total_words - mic_occ_info.unknown_total_words)
        as f64
        / info.total_words as f64;
    let unknown_chars_total_after = (info.unknown_total_chars - mic_occ_info.unknown_total_chars)
        as f64
        / info.total_chars as f64;
    let unknown_words_unique_after = (info.unknown_unique_words - mic_occ_info.unknown_unique_words)
        as f64
        / info.unique_words as f64;
    let unknown_chars_unique_after = (info.unknown_unique_chars - mic_occ_info.unknown_unique_chars)
        as f64
        / info.unique_chars as f64;
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
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Known before/after learning"),
        )
        .widths(&[
            Constraint::Percentage(34),
            Constraint::Percentage(33),
            Constraint::Percentage(33),
        ])
}

pub fn draw_centered_input(
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

pub fn get_centered_rect(r: Rect) -> Rect {
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

// convert text to list of spans of wrapped text based on given area
// see split_to_lines for prefix
pub fn get_wrapping_spans<'a>(s: &'a str, area: &Rect, prefix: Option<&'a str>) -> Vec<Spans<'a>> {
    assert!(area.width > 2);
    split_to_lines(s, (area.width - 2) as usize, prefix)
        .into_iter()
        .map(|line| Spans::from(vec![Span::raw(line)]))
        .collect()
}

// split messages to fit the width of the UI panel to prevent overflow over the UI bounds
// if not None, add prefix to first line and indent next lines
pub fn split_to_lines(input: &str, width: usize, prefix: Option<&str>) -> Vec<String> {
    let mut split = Vec::with_capacity(input.width_cjk() / width);
    let mut row = String::new();
    let mut index = 0;
    let mut is_first_line = true;

    // depending on prefix arg, calculate actual text width and indent of next lines
    let (text_width, prefix_first_line, prefix_next_lines) = match prefix {
        Some(p) => {
            let prefix_width = p.width_cjk();
            (
                width - prefix_width,
                p.to_string(),
                " ".repeat(prefix_width),
            )
        }
        None => (width, "".to_string(), "".to_string()),
    };

    // transforming line to prefixed line
    let get_prefixed_line = |row: &mut String, is_first_line: bool| {
        if is_first_line {
            format!("{}{}", prefix_first_line, &std::mem::take(row))
        } else {
            format!("{}{}", prefix_next_lines, &std::mem::take(row))
        }
    };

    for current_char in input.graphemes(true).collect::<Vec<&str>>() {
        // if adding a character would go out of bounds, create new line
        if (index != 0 && index == text_width)
            || current_char == "\n"
            || index + current_char.width_cjk() > text_width
        {
            split.push(get_prefixed_line(&mut row, is_first_line));
            index = 0;
            is_first_line = false;
        }

        // ignore new line character, already accounted for by splitting
        if current_char != "\n" {
            row.push_str(current_char);
        }
        index += current_char.width_cjk();
    }

    // last line
    if !row.is_empty() {
        split.push(get_prefixed_line(&mut row, is_first_line));
    }
    split
}
