#[derive(Clone)]
pub struct ParsedTest {
    pub test_name: String,
    pub row_location: usize,
    pub full_path: String,
}

