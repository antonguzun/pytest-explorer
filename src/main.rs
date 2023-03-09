use ansi_to_tui::IntoText;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use pytexp::parser;
use pytexp::logs::emit_error;
use std::{cmp::min, process::Command};
use std::{error::Error, io};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use unicode_width::UnicodeWidthStr;

enum InputMode {
    TestScrolling,
    OutputScrolling,
    FilterEditing,
}

struct App {
    input: String,
    input_mode: InputMode,
    test_stdout: String,
    stdout_cursor: usize,
    tests: Vec<String>,
    test_cursor: usize,
}

impl App {
    fn new(tests:  Vec<String>)-> App {
        App {
            input: String::new(),
            input_mode: InputMode::TestScrolling,
            test_stdout: String::new(),
            stdout_cursor: 0,
            tests,
            test_cursor: 0,
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let pytest_tree = parser::run()?;
    // create app and run it
    emit_error(&format!("tests load {}\n", pytest_tree.len()));
    let app = App::new(pytest_tree);

    let res = run_app(&mut terminal, app);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}")
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &app))?;

        if let Event::Key(key) = event::read()? {
            match app.input_mode {
                InputMode::TestScrolling => match key.code {
                    KeyCode::Char('2') => {
                        app.input_mode = InputMode::OutputScrolling;
                    }
                    KeyCode::Char('f') => {
                        app.input_mode = InputMode::FilterEditing;
                    }
                    KeyCode::Char('q') => {
                        return Ok(());
                    }

                    KeyCode::Up => {
                        if app.test_cursor > 0 {
                            app.test_cursor -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if app.test_cursor
                            < min(usize::MAX, app.tests.len()).saturating_sub(1)
                        {
                            app.test_cursor += 1;
                        }
                    }
                    KeyCode::Enter => {
                        let filters: Vec<String> = app
                            .input
                            .trim()
                            .clone()
                            .split(' ')
                            .map(String::from)
                            .collect();
                        match app
                            .tests
                            .iter()
                            .filter(|t| {
                                filters
                                    .clone()
                                    .into_iter()
                                    .any(|f| t.contains(&f))
                            }).cloned()
                            .collect::<Vec<String>>()
                            .get(app.test_cursor)
                        {
                            Some(test_name) => {
                                let output = Command::new("pytest")
                                    .arg(test_name)
                                    .arg("-vvv")
                                    .arg("-p")
                                    .arg("no:warnings")
                                    .env("PYTEST_ADDOPTS", "--color=yes")
                                    .output()
                                    .expect("failed to execute process");
                                let temp: String =
                                    String::from_utf8_lossy(&output.stdout).try_into().unwrap();
                                app.test_stdout = temp;
                            }
                            None => {}
                        };
                    }
                    _ => {}
                },
                InputMode::FilterEditing => match key.code {
                    KeyCode::Char(c) => {
                        app.input.push(c);
                    }
                    KeyCode::Backspace => {
                        app.input.pop();
                    }
                    KeyCode::Esc | KeyCode::Enter | KeyCode::Up | KeyCode::Down => {
                        app.input_mode = InputMode::TestScrolling;
                    }
                    _ => {}
                },
                InputMode::OutputScrolling => match key.code {
                    KeyCode::Char('1') => {
                        app.stdout_cursor = 0;
                        app.input_mode = InputMode::TestScrolling;
                    }
                    KeyCode::Char('f') => {
                        app.input_mode = InputMode::FilterEditing;
                    }
                    KeyCode::Char('q') => {
                        return Ok(());
                    }
                    KeyCode::Up => {
                        app.stdout_cursor = app.stdout_cursor.saturating_sub(5);
                    }
                    KeyCode::Down => {
                        app.stdout_cursor = app.stdout_cursor.saturating_add(5);
                    }
                    _ => {}
                },
            }
        }
    }
}
fn draw_help<B: Backend>(f: &mut Frame<B>, app: &App, area: Rect) {
    let (msg, style) = match app.input_mode {
        InputMode::TestScrolling | InputMode::OutputScrolling => (
            vec![
                Span::raw("Press "),
                Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to exit, "),
                Span::styled("f", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to filter."),
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
    f.render_widget(input, area);
    match app.input_mode {
        InputMode::FilterEditing => f.set_cursor(area.x + app.input.width() as u16 + 1, area.y + 1),
        _ => {}
    }
}
fn draw_test_with_output<B: Backend>(f: &mut Frame<B>, app: &App, area: Rect) {
    let constraints = vec![Constraint::Percentage(50), Constraint::Percentage(50)];
    let chunks = Layout::default()
        .constraints(constraints)
        .direction(Direction::Horizontal)
        .split(area);
    let start_task_list = app.test_cursor.saturating_sub(area.height as usize / 2);
    let filters: Vec<String> = app
        .input
        .clone()
        .split(' ')
        .map(String::from)
        .collect();
    let messages: Vec<ListItem> = app
        .tests
        .iter()
        .filter(|t| {
            filters
                .clone()
                .into_iter()
                .any(|f| t.contains(&f))
        })
        .enumerate()
        .filter(|(i, _)| i >= &start_task_list && i < &(start_task_list + area.height as usize))
        .map(|(i, t)| {
            let content = vec![Spans::from(Span::raw(t.to_string()))];
            if i == app.test_cursor.try_into().unwrap() {
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

fn ui<B: Backend>(f: &mut Frame<B>, app: &App) {
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
        .split(f.size());

    draw_help(f, app, chunks[0]);
    draw_filter_input(f, app, chunks[1]);
    draw_test_with_output(f, app, chunks[2]);
}
