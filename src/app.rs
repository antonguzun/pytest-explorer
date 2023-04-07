use crate::entities::ParsedTest;
use std::cmp::min;

pub enum InputMode {
    TestScrolling,
    OutputScrolling,
    FilterEditing,
    ErrorMessage,
}

pub struct App {
    pub input: String,
    pub input_mode: InputMode,
    pub test_stdout: String,
    pub stdout_cursor: usize,
    pub tests: Vec<ParsedTest>,
    pub filtered_tests_count: usize,
    pub test_cursor: usize,
    pub loading_lock: bool,
    pub error_message: String,
}

impl App {
    pub fn new(tests: Vec<ParsedTest>) -> App {
        App {
            input: String::new(),
            input_mode: InputMode::TestScrolling,
            test_stdout: String::new(),
            stdout_cursor: 0,
            tests,
            filtered_tests_count: 0,
            test_cursor: 0,
            loading_lock: false,
            error_message: String::new(),
        }
    }
    pub fn load_filters_from_app(&self) -> Vec<String> {
        self.input.trim().split(' ').map(String::from).collect()
    }

    pub fn is_accure_all_filters(filters: &[String], t: &str) -> bool {
        filters.to_owned().iter().cloned().all(|f| t.contains(&f))
    }

    pub fn find_selected_test(&self) -> Option<ParsedTest> {
        let filters = self.load_filters_from_app();
        self.tests
            .iter()
            .filter(|t| App::is_accure_all_filters(&filters, &t.full_path))
            .nth(self.test_cursor)
            .cloned()
    }

    pub fn update_filtered_test_count(&mut self) {
        let filters = self.load_filters_from_app();
        self.filtered_tests_count = self
            .tests
            .iter()
            .filter(|t| App::is_accure_all_filters(&filters, &t.full_path))
            .count();
        self.test_cursor = min(
            self.test_cursor,
            self.filtered_tests_count.saturating_sub(1),
        );
    }

    pub fn set_error(&mut self, err: anyhow::Error) {
        self.error_message = err.to_string();
        self.input_mode = InputMode::ErrorMessage;
    }
    pub fn clean_error(&mut self){
        self.input_mode = InputMode::TestScrolling;
        self.error_message = String::new();
    }
}
