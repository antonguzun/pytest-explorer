use ansi_to_tui::IntoText;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use pytexp::parser;
use std::{cmp::min, process::Command};
use std::{error::Error, io};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
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
    filtered_tests_count: usize,
    test_cursor: usize,
    loading_lock: bool,
}

impl App {
    fn new(tests: Vec<String>) -> App {
        App {
            input: String::new(),
            input_mode: InputMode::TestScrolling,
            test_stdout: String::new(),
            stdout_cursor: 0,
            tests,
            filtered_tests_count: 0,
            test_cursor: 0,
            loading_lock: false,
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

    let mut app = App::new(vec![]);
    app.loading_lock = true;
    terminal.draw(|f| ui(f, &app))?;
    app.tests = parser::run()?;
    app.filtered_tests_count = app.tests.len();
    app.loading_lock = false;
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

fn load_filters_from_app(app: &App) -> Vec<String> {
    app.input.trim().split(' ').map(String::from).collect()
}

fn is_accure_all_filters(filters: &[String], t: &str) -> bool {
    filters.to_owned().iter().cloned().all(|f| t.contains(&f))
}

fn find_selected_test(app: &App) -> Option<String> {
    let filters = load_filters_from_app(app);
    app.tests
        .iter()
        .filter(|t| is_accure_all_filters(&filters, t))
        .cloned()
        .collect::<Vec<String>>()
        .get(app.test_cursor)
        .map(|s| s.to_string())
}

fn update_filtered_test_count(app: &mut App) {
    let filters = load_filters_from_app(&*app);
    app.filtered_tests_count = app
        .tests
        .iter()
        .filter(|t| is_accure_all_filters(&filters, t))
        .count();
    app.test_cursor = min(app.test_cursor, app.filtered_tests_count.saturating_sub(1));
}

fn run_command_in_shell(command: &str) {
    Command::new("gnome-terminal")
        .arg("--title=newWindow")
        .arg("--")
        .arg("zsh")
        .arg("-c")
        .arg(command)
        .spawn()
        .expect("run test in terminal command failed to start");
}

fn run_test(test_name: String) -> std::process::Output {
    let output = Command::new("pytest")
        .arg(test_name)
        .arg("-vvv")
        .arg("-p")
        .arg("no:warnings")
        .env("PYTEST_ADDOPTS", "--color=yes")
        .output()
        .expect("failed to execute process");
    output
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &app))?;
        let size = terminal.size()?;
        let half_of_height = (size.height / 2) as usize;

        if let Event::Key(key) = event::read()? {
            match app.input_mode {
                InputMode::TestScrolling => match key.code {
                    KeyCode::Char('2') | KeyCode::Char('l') | KeyCode::Right => {
                        app.input_mode = InputMode::OutputScrolling;
                    }
                    KeyCode::Char('f') => {
                        app.input_mode = InputMode::FilterEditing;
                    }
                    KeyCode::Char('q') => {
                        return Ok(());
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        app.test_cursor = app.test_cursor.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        app.test_cursor = min(
                            app.test_cursor.saturating_add(1),
                            app.filtered_tests_count - 1,
                        );
                    }
                    KeyCode::PageUp => {
                        app.test_cursor = app.test_cursor.saturating_sub(half_of_height);
                    }
                    KeyCode::PageDown => {
                        app.test_cursor = min(
                            app.test_cursor.saturating_add(half_of_height),
                            app.filtered_tests_count - 1,
                        );
                    }
                    KeyCode::Home => {
                        app.test_cursor = 0;
                    }
                    KeyCode::End => {
                        app.test_cursor = app.filtered_tests_count - 1;
                    }
                    KeyCode::Enter => {
                        if let Some(test_name) = find_selected_test(&app) {
                            app.loading_lock = true;
                            terminal.draw(|f| ui(f, &app))?;
                            let output = run_test(test_name);

                            if !output.stdout.is_empty() {
                                let temp: String =
                                    String::from_utf8_lossy(&output.stdout).try_into().unwrap();
                                app.test_stdout = temp;
                                app.loading_lock = false;
                            } else {
                                let temp: String =
                                    String::from_utf8_lossy(&output.stderr).try_into().unwrap();
                                app.test_stdout = temp;
                                app.loading_lock = false;
                            }
                        };
                    }
                    KeyCode::Char('r') => {
                        if let Some(test_name) = find_selected_test(&app) {
                            let command =
                                format!("pytest {test_name} -vvv -p no:warnings; exec zsh");
                            run_command_in_shell(&command);
                        }
                    }
                    KeyCode::Char('o') => {
                        // gnome-terminal --title=newTab -- zsh -c "${EDITOR} Cargo.toml"
                        if let Some(test_name) = find_selected_test(&app) {
                            let command =
                                format!("${} {}", "EDITOR", test_name.split("::").next().unwrap());
                            run_command_in_shell(&command);
                        }
                    }
                    _ => {}
                },
                InputMode::FilterEditing => match key.code {
                    KeyCode::Char(c) => {
                        update_filtered_test_count(&mut app);
                        app.input.push(c);
                    }
                    KeyCode::Backspace => {
                        update_filtered_test_count(&mut app);
                        app.input.pop();
                    }
                    KeyCode::Esc
                    | KeyCode::Enter
                    | KeyCode::Up
                    | KeyCode::Down
                    | KeyCode::End
                    | KeyCode::Home
                    | KeyCode::Tab
                    | KeyCode::PageDown
                    | KeyCode::PageUp => {
                        app.input_mode = InputMode::TestScrolling;
                    }
                    _ => {}
                },
                InputMode::OutputScrolling => match key.code {
                    KeyCode::Char('1') | KeyCode::Char('h') | KeyCode::Left => {
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
                    KeyCode::PageUp => {
                        app.stdout_cursor = app.stdout_cursor.saturating_sub(half_of_height);
                    }
                    KeyCode::PageDown => {
                        app.stdout_cursor = min(
                            app.stdout_cursor.saturating_add(half_of_height),
                            app.test_stdout.lines().count().saturating_sub(51),
                        );
                    }
                    KeyCode::Home => {
                        app.stdout_cursor = 0;
                    }
                    KeyCode::End => {
                        app.stdout_cursor = app.test_stdout.lines().count().saturating_sub(51);
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
    let filters = load_filters_from_app(app);

    let messages: Vec<ListItem> = app
        .tests
        .iter()
        .filter(|t| is_accure_all_filters(&filters.clone(), t))
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

fn ui<B: Backend>(f: &mut Frame<B>, app: &App) {
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
