use crate::app::{App, InputMode};
use ansi_to_tui::IntoText;
use std::cmp::min;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};
use unicode_width::UnicodeWidthStr;

fn draw_help<B: Backend>(f: &mut Frame<B>, app: &App, area: Rect) {
    let (msg, style) = match app.input_mode {
        InputMode::TestScrolling | InputMode::OutputScrolling => (
            vec![
                Span::raw("Press "),
                Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to exit, "),
                Span::styled("f", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to filter, "),
                Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to run test, "),
                Span::styled("r", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to run test in new shell, "),
                Span::styled("hjkl", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" or arrows to navigate, "),
                Span::styled("2", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to activate Output, "),
                Span::styled("o", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to open file with test."),
            ],
            Style::default(),
        ),
        InputMode::FilterEditing => (
            vec![
                Span::raw("Press "),
                Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to stop editing, "),
                Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to record the message"),
            ],
            Style::default(),
        ),
    };
    let mut text = Text::from(Spans::from(msg));
    text.patch_style(style);
    let help_message = Paragraph::new(text);
    f.render_widget(help_message, area);
}

fn draw_filter_input<B: Backend>(f: &mut Frame<B>, app: &App, area: Rect) {
    let input = Paragraph::new(app.input.as_ref())
        .style(match app.input_mode {
            InputMode::FilterEditing => Style::default().fg(Color::Yellow),
            _ => Style::default(),
        })
        .block(Block::default().borders(Borders::ALL).title("Filter"));
    let count = Paragraph::new(format!("{}/{}", app.filtered_tests_count, app.tests.len()))
        .alignment(tui::layout::Alignment::Right)
        .style(match app.input_mode {
            InputMode::FilterEditing => Style::default().fg(Color::Yellow),
            _ => Style::default(),
        })
        .block(Block::default().borders(Borders::ALL).title("Filter"));
    f.render_widget(input, area);
    f.render_widget(count, area);
    if let InputMode::FilterEditing = app.input_mode {
        f.set_cursor(area.x + app.input.width() as u16 + 1, area.y + 1)
    }
}
fn draw_test_with_output<B: Backend>(f: &mut Frame<B>, app: &App, area: Rect) {
    let constraints = vec![Constraint::Percentage(50), Constraint::Percentage(50)];
    let chunks = Layout::default()
        .constraints(constraints)
        .direction(Direction::Horizontal)
        .split(area);
    let start_task_list = app.test_cursor.saturating_sub(area.height as usize / 2);
    let filters = app.load_filters_from_app();

    let messages: Vec<ListItem> = app
        .tests
        .iter()
        .filter(|t| App::is_accure_all_filters(&filters.clone(), t))
        .enumerate()
        .filter(|(i, _)| i >= &start_task_list && i < &(start_task_list + area.height as usize))
        .map(|(i, t)| {
            let content = vec![Spans::from(Span::raw(t.to_string()))];
            if i == app.test_cursor {
                ListItem::new(content).style(Style::default().bg(Color::Yellow))
            } else {
                ListItem::new(content)
            }
        })
        .collect();
    let test_style = match app.input_mode {
        InputMode::TestScrolling => Style::default().fg(Color::Yellow),
        _ => Style::default(),
    };
    let messages = List::new(messages).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(test_style)
            .title("Tests"),
    );
    f.render_widget(messages, chunks[0]);

    let text = app.test_stdout.clone().into_text().unwrap();
    let start_stdout_list = min(app.stdout_cursor, text.lines.len());
    let stop_stdout_list = min(start_stdout_list + area.height as usize, text.lines.len());
    let text_to_show = text.lines[start_stdout_list..stop_stdout_list].to_vec();
    let test_style = match app.input_mode {
        InputMode::OutputScrolling => Style::default().fg(Color::Yellow),
        _ => Style::default(),
    };
    let test_outout = Paragraph::new(text_to_show).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(test_style)
            .title("Output"),
    );
    f.render_widget(test_outout, chunks[1]);
}

fn draw_loading<B: Backend>(f: &mut Frame<B>, _: &App, area: Rect) {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage(50),
                Constraint::Length(5),
                Constraint::Percentage(50),
            ]
            .as_ref(),
        )
        .split(area);

    let loading_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage(40),
                Constraint::Length(30),
                Constraint::Percentage(60),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1];
    let block = Paragraph::new("Loading ...")
        .alignment(tui::layout::Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(Clear, loading_area); //this clears out the background
    f.render_widget(block, loading_area);
}

pub fn ui<B: Backend>(f: &mut Frame<B>, app: &App) {
    let size = f.size();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(
            [
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Min(1),
            ]
            .as_ref(),
        )
        .split(size);

    draw_help(f, app, chunks[0]);
    draw_filter_input(f, app, chunks[1]);
    draw_test_with_output(f, app, chunks[2]);
    if app.loading_lock {
        draw_loading(f, app, size);
    }
}
