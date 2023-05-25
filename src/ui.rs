use crate::app::{App, InputMode};
use ansi_to_tui::IntoText;
use std::cmp::min;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};
use unicode_width::UnicodeWidthStr;

fn draw_help<B: Backend>(f: &mut Frame<B>, app: &App, area: Rect) {
    let (msg, style) = match app.input_mode {
        InputMode::TestScrolling => (
            vec![
                Span::raw("EXIT "),
                Span::styled("q ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("| FILTER "),
                Span::styled("f ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("| RUN TEST "),
                Span::styled("Enter ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("| RUN TEST IN SHELL "),
                Span::styled("r ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("| NAVIGATE "),
                Span::styled(
                    "hjkl/arrows PgUp/PgDown/Home/End ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw("| ACTIVATE OUTPUT "),
                Span::styled("2 ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("| OPEN FILE "),
                Span::styled("o", Style::default().add_modifier(Modifier::BOLD)),
            ],
            Style::default(),
        ),
        InputMode::OutputScrolling => (
            vec![
                Span::raw("EXIT "),
                Span::styled("q ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("| FILTER "),
                Span::styled("f ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("| NAVIGATE "),
                Span::styled(
                    "hjkl/arrows PgUp/PgDown/Home/End ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw("| ACTIVATE TESTS LIST "),
                Span::styled("1 ", Style::default().add_modifier(Modifier::BOLD)),
            ],
            Style::default(),
        ),
        InputMode::FilterEditing => (
            vec![
                Span::raw("STOP EDITING "),
                Span::styled("Esc/Enter ", Style::default().add_modifier(Modifier::BOLD)),
            ],
            Style::default(),
        ),
        InputMode::ErrorMessage => (
            vec![
                Span::raw("CLOSE ERROR MESSAGE "),
                Span::styled("Esc/Ente/q ", Style::default().add_modifier(Modifier::BOLD)),
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
        .filter(|t| App::is_accure_all_filters(&filters.clone(), &t.full_path))
        .enumerate()
        .filter(|(i, _)| i >= &start_task_list && i < &(start_task_list + area.height as usize))
        .map(|(i, t)| {
            let content;
            let test_line_width = chunks[0].width.saturating_sub(2);  // sub 2 cause of borders
            if t.full_path.len() > test_line_width.into() {
                content = t.full_path.chars()
                    .collect::<Vec<char>>()
                    .chunks(test_line_width.into())
                    .map(|c| Spans::from(Span::raw(c.clone().iter().collect::<String>())))
                    .collect();
            } else {
                content = vec![Spans::from(Span::raw(&t.full_path))];
            }

            if i == app.test_cursor {
                ListItem::new(content).style(Style::default().fg(Color::Black).bg(Color::Yellow))
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
    let text_to_show = Text::from(text.lines[start_stdout_list..stop_stdout_list].to_vec());
    let test_style = match app.input_mode {
        InputMode::OutputScrolling => Style::default().fg(Color::Yellow),
        _ => Style::default(),
    };
    let test_output = Paragraph::new(text_to_show)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(test_style)
                .title("Output"),
        )
        .wrap(Wrap { trim: true });
    f.render_widget(test_output, chunks[1]);
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

fn draw_error<B: Backend>(f: &mut Frame<B>, app: &App, area: Rect) {
    let error_width = 70;
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage(50),
                Constraint::Length(10),
                Constraint::Percentage(50),
            ]
            .as_ref(),
        )
        .split(area);

    let loading_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage(30),
                Constraint::Length(error_width),
                Constraint::Percentage(70),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1];
    let lines = app
        .error_message
        .clone()
        .chars()
        .collect::<Vec<char>>()
        .chunks(error_width.saturating_sub(1).into())
        .map(|c| c.iter().collect::<String>())
        .map(|s| Spans::from(s))
        .collect::<Vec<Spans>>();
    let style = Style::default().fg(Color::Red);
    let block = Paragraph::new(lines).block(
        Block::default()
            .title("Error")
            .borders(Borders::ALL)
            .border_style(style),
    );
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
    if !app.error_message.is_empty() {
        draw_error(f, app, size);
    }
}
