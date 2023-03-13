use std::cmp::min;

pub enum InputMode {
    TestScrolling,
    OutputScrolling,
    FilterEditing,
}

pub struct App {
    pub input: String,
    pub input_mode: InputMode,
    pub test_stdout: String,
    pub stdout_cursor: usize,
    pub tests: Vec<String>,
    pub filtered_tests_count: usize,
    pub test_cursor: usize,
    pub loading_lock: bool,
}

impl App {
    pub fn new(tests: Vec<String>) -> App {
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
    pub fn load_filters_from_app(self: &Self) -> Vec<String> {
        self.input.trim().split(' ').map(String::from).collect()
    }

    pub fn is_accure_all_filters(filters: &[String], t: &str) -> bool {
        filters.to_owned().iter().cloned().all(|f| t.contains(&f))
    }

    pub fn find_selected_test(self: &Self) -> Option<String> {
        let filters = self.load_filters_from_app();
        self.tests
            .iter()
            .filter(|t| App::is_accure_all_filters(&filters, t))
            .cloned()
            .collect::<Vec<String>>()
            .get(self.test_cursor)
            .map(|s| s.to_string())
    }

    pub fn update_filtered_test_count(self: &mut Self) {
        let filters = self.load_filters_from_app();
        self.filtered_tests_count = self
            .tests
            .iter()
            .filter(|t| App::is_accure_all_filters(&filters, t))
            .count();
        self.test_cursor = min(
            self.test_cursor,
            self.filtered_tests_count.saturating_sub(1),
        );
    }
}
