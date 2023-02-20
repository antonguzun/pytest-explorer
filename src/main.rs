use ansi_to_tui::IntoText;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
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
    Normal,
    Editing,
}

/// App holds the state of the application
struct App {
    /// Current value of the input box
    input: String,
    /// Current input mode
    input_mode: InputMode,
    /// History of recorded messages
    messages: Vec<String>,
    test_stdout: String,
    tests: Vec<String>,
    test_cursor: usize,
}

impl App {
    fn new(tests: Vec<String>) -> App {
        App {
            input: String::new(),
            input_mode: InputMode::Normal,
            messages: Vec::new(),
            test_stdout: String::new(),
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

    let output = Command::new("pytest")
        .arg("--collect-only")
        .arg("-q")
        .arg("-p")
        .arg("no:warnings")
        .output()
        .expect("failed to execute process");
    let temp: String = String::from_utf8_lossy(&output.stdout).try_into().unwrap();
    let mut tests: Vec<String> = temp
        .split("\n")
        .map(|v| String::from(v))
        .filter(|v| v.contains("test"))
        .collect();
    tests.pop();
    // create app and run it
    let app = App::new(tests);

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
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &app))?;

        if let Event::Key(key) = event::read()? {
            match app.input_mode {
                InputMode::Normal => match key.code {
                    KeyCode::Char('f') => {
                        app.input_mode = InputMode::Editing;
                    }
                    KeyCode::Char('q') => {
                        return Ok(());
                    }

                    KeyCode::Up => {
                        if app.test_cursor > 0 {
                            app.test_cursor = app.test_cursor - 1;
                        }
                    }
                    KeyCode::Down => {
                        if app.test_cursor < min(usize::MAX, app.tests.len()) - 1 {
                            app.test_cursor = app.test_cursor + 1;
                        }
                    }
                    KeyCode::Enter => {
                        let filters: Vec<String> = app
                            .input
                            .clone()
                            .split(" ")
                            .map(|s| String::from(s))
                            .collect();
                        match app
                            .tests
                            .iter()
                            .filter(|m| filters.clone().into_iter().all(|f| m.contains(&f)) )
                            .collect::<Vec<&String>>()
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
                InputMode::Editing => match key.code {
                    KeyCode::Char(c) => {
                        app.input.push(c);
                    }
                    KeyCode::Backspace => {
                        app.input.pop();
                    }
                    KeyCode::Esc | KeyCode::Enter | KeyCode::Up | KeyCode::Down => {
                        app.input_mode = InputMode::Normal;
                    }
                    _ => {}
                },
            }
        }
    }
}
fn draw_help<B: Backend>(f: &mut Frame<B>, app: &App, area: Rect) {
    let (msg, style) = match app.input_mode {
        InputMode::Normal => (
            vec![
                Span::raw("Press "),
                Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to exit, "),
                Span::styled("f", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to filter."),
            ],
            Style::default(),
        ),
        InputMode::Editing => (
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
            InputMode::Normal => Style::default(),
            InputMode::Editing => Style::default().fg(Color::Yellow),
        })
        .block(Block::default().borders(Borders::ALL).title("Filter"));
    f.render_widget(input, area);
    match app.input_mode {
        InputMode::Normal =>
            // Hide the cursor. `Frame` does this by default, so we don't need to do anything here
            {}

        InputMode::Editing => {
            // Make the cursor visible and ask tui-rs to put it at the specified coordinates after rendering
            f.set_cursor(
                // Put cursor past the end of the input text
                area.x + app.input.width() as u16 + 1,
                // Move one line down, from the border to the input line
                area.y + 1,
            )
        }
    }
}
fn draw_test_with_output<B: Backend>(f: &mut Frame<B>, app: &App, area: Rect) {
    let constraints = vec![Constraint::Percentage(50), Constraint::Percentage(50)];
    let chunks = Layout::default()
        .constraints(constraints)
        .direction(Direction::Horizontal)
        .split(area);
    let start_task_list = app.test_cursor.saturating_sub(area.height as usize / 2);
    // println!("P{}", start_task_list);
    let filters: Vec<String> = app
        .input
        .clone()
        .split(" ")
        .map(|s| String::from(s))
        .collect();
    let messages: Vec<ListItem> = app
        // .tests[start_task_list:start_task_list + area.height]
        .tests
        .iter()
        .filter(|m| filters.clone().into_iter().all(|f| m.contains(&f)))
        .enumerate()
        .filter(|(i, _)| i >= &start_task_list && i < &(start_task_list + area.height as usize))
        .map(|(i, m)| {
            let content = vec![Spans::from(Span::raw(format!("{}: {}", i, m)))];
            if i == app.test_cursor.try_into().unwrap() {
                ListItem::new(content).style(Style::default().bg(Color::Yellow))
            } else {
                ListItem::new(content)
            }
        })
        .collect();
    let messages = List::new(messages).block(Block::default().borders(Borders::ALL).title("Tests"));
    // println!("{}", area.height);
    f.render_widget(messages, chunks[0]);

    // let text = Text::from(app.test_stdout.clone());

    let text = app.test_stdout.clone().into_text().unwrap();
    let test_outout =
        Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Output"));
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
