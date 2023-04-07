use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use pytexp::app::{App, InputMode};
use pytexp::external_calls;
use pytexp::parser;
use pytexp::ui::ui;
use std::cmp::min;
use std::{error::Error, io};
use tui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Collect test without running ui
    #[arg(short, long, action)]
    collect_only: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    if args.collect_only {
        let tests = parser::run()?;
        let tests_count = tests.len();
        for i in tests {
            println!("{}", i.test_name);
        }
        println!("collected {tests_count} tests");
        return Ok(());
    }

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
                            app.filtered_tests_count.saturating_sub(1),
                        );
                    }
                    KeyCode::PageUp => {
                        app.test_cursor = app.test_cursor.saturating_sub(half_of_height);
                    }
                    KeyCode::PageDown => {
                        app.test_cursor = min(
                            app.test_cursor.saturating_add(half_of_height),
                            app.filtered_tests_count.saturating_sub(1),
                        );
                    }
                    KeyCode::Home => {
                        app.test_cursor = 0;
                    }
                    KeyCode::End => {
                        app.test_cursor = app.filtered_tests_count.saturating_sub(1);
                    }
                    KeyCode::Enter => {
                        if let Some(test) = app.find_selected_test() {
                            app.loading_lock = true;
                            terminal.draw(|f| ui(f, &app))?;
                            let output = external_calls::run_test(test.full_path);

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
                        if let Some(test) = app.find_selected_test() {
                            let command =
                                format!("pytest {} -vvv -p no:warnings; exec zsh", test.full_path);
                            if let Err(err) = external_calls::run_command_in_shell(&command) {
                                app.set_error(err)
                            }
                        }
                    }
                    KeyCode::Char('o') => {
                        // gnome-terminal --title=newTab -- zsh -c "${EDITOR} Cargo.toml"
                        if let Some(test) = app.find_selected_test() {
                            if let Err(m) = external_calls::open_editor(&test) {
                                app.set_error(m)
                            };
                        }
                    }
                    _ => {}
                },
                InputMode::FilterEditing => match key.code {
                    KeyCode::Char(c) => {
                        app.update_filtered_test_count();
                        app.input.push(c);
                    }
                    KeyCode::Backspace => {
                        app.update_filtered_test_count();
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
                InputMode::ErrorMessage => match key.code {
                    KeyCode::Esc
                    | KeyCode::Enter
                    | KeyCode::Char('q')
                    | KeyCode::Up
                    | KeyCode::Down
                    | KeyCode::End
                    | KeyCode::Home
                    | KeyCode::Tab
                    | KeyCode::PageDown
                    | KeyCode::PageUp => app.clean_error(),
                    _ => {}
                },
            }
        }
    }
}
