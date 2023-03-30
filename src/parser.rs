use std::path::PathBuf;

use anyhow::Result;
use rustpython_parser::ast;
use rustpython_parser::parse_program;
use walkdir::WalkDir;

use crate::entities::ParsedTest;

impl ParsedTest {
    fn new(name: String, location: &ast::Location, filepath: &str) -> Self {
        ParsedTest {
            test_name: name.clone(),
            row_location: location.row(),
            full_path: format!("{filepath}::{name}"),
        }
    }
}

pub fn parse_file(path: PathBuf) -> Result<Vec<ParsedTest>> {
    let contents = std::fs::read_to_string(&path)?;
    let filepath = path.to_str().unwrap();
    let python_ast = parse_program(&contents, "<embedded>")?;
    let mut tests = vec![];
    for i in python_ast {
        let ast::Located { node, location, .. } = i;
        match node {
            ast::StmtKind::FunctionDef { name, .. } => {
                if name.starts_with("test_") {
                    let test = ParsedTest::new(name, &location, filepath);
                    tests.push(test);
                }
            }
            ast::StmtKind::AsyncFunctionDef { name, .. } => {
                if name.starts_with("test_") {
                    let test = ParsedTest::new(name, &location, filepath);
                    tests.push(test);
                }
            }
            ast::StmtKind::ClassDef {
                name: class_name,
                bases: _, //FIXME! add tests from bases
                body,
                ..
            } => {
                add_class(class_name, body, &mut tests, filepath, &location);
            }
            _ => {}
        }
    }
    Ok(tests)
}

fn add_class(
    class_name: String,
    body: Vec<ast::Located<ast::StmtKind>>,
    input: &mut Vec<ParsedTest>,
    filepath: &str,
    class_location: &ast::Location,
) {
    if class_name.starts_with("Test") {
        let mut tests_in_class = vec![];
        for m in body {
            let ast::Located {
                node: m_node,
                location,
                ..
            } = m;
            match m_node {
                ast::StmtKind::FunctionDef { name, .. } => {
                    if name.starts_with("test_") {
                        let test = ParsedTest::new(format!("{class_name}::{name}"), &location, filepath);
                        tests_in_class.push(test);
                    }
                }
                ast::StmtKind::AsyncFunctionDef { name, .. } => {
                    if name.starts_with("test_") {
                        let test = ParsedTest::new(format!("{class_name}::{name}"), &location, filepath);
                        tests_in_class.push(test);
                    }
                }
                _ => (),
            }
        }
        if !tests_in_class.is_empty() {
            let class = ParsedTest::new(class_name, class_location, filepath);
            input.push(class);

            input.extend(tests_in_class);
        }
    }
}

pub fn run() -> Result<Vec<ParsedTest>> {
    let mut res = vec![];
    for entry in WalkDir::new("tests")
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let f_name = entry.file_name().to_string_lossy();
        if f_name.ends_with(".py") & (f_name.ends_with("_test.py") | f_name.starts_with("test_")) {
            let path = entry.into_path();
            let parsed_tests = parse_file(path.clone())?;
            res.extend(parsed_tests);
        }
    }
    Ok(res)
}

#[cfg(test)]
mod tests {
    use crate::parser;
    #[test]
    fn test_file_parsing() {
        let python_source = r#"
def test_one(a: int, b:int):
    return a + b

class GroupedTests:
    def test_groupped(a: int, b:int):
        return a + b

def test_two(a: int, b:int):
    return a + b

        "#;
        let k = parser::parse_file(python_source)
            .unwrap()
            .iter()
            .map(|o| o.test_name)
            .collect();

        assert_eq!(
            k,
            vec![
                "test_one".to_string(),
                "GroupedTests".to_string(),
                "test_groupped".to_string(),
                "test_two".to_string()
            ]
        );
    }
}
