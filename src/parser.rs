use anyhow::Result;
use rustpython_parser::ast;
use rustpython_parser::parse_program;
use walkdir::WalkDir;

pub fn parse_file(contents: &str) -> Result<Vec<String>> {
    let python_ast = parse_program(contents, "<embedded>")?;
    let mut tests = vec![];
    for i in python_ast {
        let ast::Located { node, .. } = i;
        match node {
            ast::StmtKind::FunctionDef { name, .. } => {
                add_function(name, &mut tests);
            }
            ast::StmtKind::AsyncFunctionDef { name, .. } => {
                add_function(name, &mut tests);
            }
            ast::StmtKind::ClassDef {
                name: class_name,
                bases: _, //FIXME! add tests from bases
                body,
                ..
            } => {
                add_class(class_name, body, &mut tests);
            }
            _ => {}
        }
    }
    Ok(tests)
}

fn add_class(class_name: String, body: Vec<ast::Located<ast::StmtKind>>, input: &mut Vec<String>) {
    if class_name.starts_with("Test") {
        let mut tests_in_class = vec![];
        for m in body {
            let ast::Located { node: m_node, .. } = m;
            match m_node {
                ast::StmtKind::FunctionDef { name, .. } => {
                    if name.starts_with("test_") {
                        tests_in_class.push(format!("{}::{}", &class_name, name));
                    }
                }
                ast::StmtKind::AsyncFunctionDef { name, .. } => {
                    if name.starts_with("test_") {
                        tests_in_class.push(format!("{}::{}", &class_name, name));
                    }
                }
                _ => (),
            }
        }
        if !tests_in_class.is_empty() {
            input.push(class_name);
            input.extend(tests_in_class);
        }
    }
}

fn add_function(name: String, input: &mut Vec<String>) {
    if name.starts_with("test_") {
        input.push(name);
    }
}

pub fn run() -> Result<Vec<String>> {
    let mut paths = vec![];
    for entry in WalkDir::new("tests")
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let f_name = entry.file_name().to_string_lossy();
        if f_name.ends_with(".py") & (f_name.ends_with("_test.py") | f_name.starts_with("test_")) {
            paths.push(entry.into_path());
        }
    }
    let mut res = vec![];
    for path in paths {
        // logs::emit_error(&path);
        let contents = std::fs::read_to_string(path.clone())?;
        parse_file(&contents)?
            .into_iter()
            .for_each(|r| res.push(format!("{}::{}", path.to_str().unwrap(), r)));
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
        let k = parser::parse_file(python_source).unwrap();

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
