use regex::Regex;
use std::process::Command;

use crate::logs::emit_error;
// use std::rc::Rc;

#[derive(Debug, Clone)]
enum PytestElementType {
    Module,
    Class,
    Function,
}

#[derive(Debug, Clone)]
pub struct PytestElement {
    id: usize,
    pub name: String,
    parent_id: usize,
    element_type: PytestElementType,
}

#[derive(Debug, Clone)]
pub struct PytestTree {
    id: usize,
    pub elements: Vec<PytestElement>,
}

impl PytestTree {
    pub fn new(id: usize, elements: Vec<PytestElement>) -> Self {
        Self { id, elements }
    }
    fn element_by_parent_id(&self, parent_id: usize) -> PytestElement {
        match self.elements.iter().find(|e| e.id == parent_id) {
            Some(p) => p.clone(),
            None => {
                emit_error(&format!("element_by_parent_id {}", parent_id));
                panic!()
            }
        }
    }
    pub fn is_test_contains_value(&self, test: PytestElement, value: &str) -> bool {
        let mut targets = vec![test.name];
        if test.parent_id != 0 {
            let parent = self.element_by_parent_id(test.parent_id);
            match parent.element_type {
                PytestElementType::Module => targets.push(parent.name),
                PytestElementType::Class => {
                    let element = self.element_by_parent_id(parent.parent_id);
                    targets.push(element.name.clone())
                }
                PytestElementType::Function => panic!("Function type cannot be parent"),
            };
        }
        targets.iter().any(|t| t.contains(value))
    }
    pub fn full_test_name(&self, test: PytestElement) -> String {
        match test.element_type {
            PytestElementType::Module => return test.name.clone(),
            PytestElementType::Class => {
                let module_name = self.element_by_parent_id(test.parent_id).name;
                return format!("{}::{}", module_name, test.name);
            }
            PytestElementType::Function => {
                let parent = self.element_by_parent_id(test.parent_id);
                if parent.parent_id == 0 {
                    return format!("{}::{}", parent.name, test.name);
                } else {
                    let top_parent = self.element_by_parent_id(parent.parent_id);
                    return format!("{}::{}::{}", top_parent.name, parent.name, test.name);
                }
            }
        }
    }
}

struct Counter {
    last_value: usize,
}
impl Counter {
    fn get(&mut self) -> usize {
        let res = self.last_value;
        self.last_value += 1;
        res
    }
    fn new() -> Self {
        Counter { last_value: 0 }
    }
}

pub fn fetch_pytest_collected_stdout() -> Result<String, String> {
    let output = Command::new("pytest")
        .arg("--collect-only")
        .arg("-p")
        .arg("no:warnings")
        .output()
        .expect("failed to execute process");
    let res = String::from_utf8_lossy(&output.stdout).try_into().unwrap();
    Ok(res)
}
pub fn collect(input: String) -> Result<PytestTree, String> {
    let mut counter = Counter::new();
    let mut elements: Vec<PytestElement> = vec![];
    let tree_id = counter.get();
    let re = Regex::new(r"^(\s*)<(Function|Class|Module)\s(.*)>$").unwrap();
    let mut last_module_id: Option<usize> = None;
    let mut last_class_in_module_id: Option<usize> = None;
    for line in input.split("\n") {
        match re.captures(line) {
            Some(caps) => {
                let ident: usize = caps.get(1).map_or("", |m| m.as_str()).len();
                let name = caps.get(3).unwrap().as_str().to_string();
                match caps.get(2).unwrap().as_str() {
                    "Function" => {
                        let e = PytestElement {
                            id: counter.get(),
                            name,
                            parent_id: match last_class_in_module_id {
                                Some(p) => p,
                                None => last_module_id.unwrap(),
                            },
                            element_type: PytestElementType::Function,
                        };
                        elements.push(e);
                    }
                    "Class" => {
                        let e = PytestElement {
                            id: counter.get(),
                            name,
                            parent_id: last_module_id.unwrap(),
                            element_type: PytestElementType::Class,
                        };
                        last_class_in_module_id = Some(e.id);
                        elements.push(e);
                    }
                    "Module" => {
                        let e = PytestElement {
                            id: counter.get(),
                            name,
                            parent_id: tree_id,
                            element_type: PytestElementType::Module,
                        };
                        last_module_id = Some(e.id);
                        last_class_in_module_id = None;
                        elements.push(e);
                    }
                    _ => {}
                };
            }
            None => {}
        };
    }
    Ok(PytestTree::new(tree_id, elements))
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::prelude::*;

    use crate::collect_test::{collect, PytestElementType};

    #[test]
    fn test() {
        let mut file = File::open("./tests/pytest_aiohttp_collection.txt").unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        let tree = collect(contents).unwrap();
        let count_of_modules = &tree
            .clone()
            .elements
            .into_iter()
            .filter(|e| match e.element_type {
                PytestElementType::Module => true,
                _ => false,
            })
            .count();
        let count_of_classes = &tree
            .clone()
            .elements
            .into_iter()
            .filter(|e| match e.element_type {
                PytestElementType::Class => true,
                _ => false,
            })
            .count();
        let count_of_functions = &tree
            .elements
            .into_iter()
            .filter(|e| match e.element_type {
                PytestElementType::Function => true,
                _ => false,
            })
            .count();
        assert!(*count_of_modules == 59);
        assert!(*count_of_classes == 38);
        assert!(*count_of_functions == 2587);
    }
    #[test]
    fn test_contains() {
        let mut file = File::open("./tests/pytest_aiohttp_collection.txt").unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        let tree = collect(contents).unwrap();
        let target_test = tree.elements.get(4).unwrap();
        assert_eq!(&target_test.name, "test_pause_reading_stub_transport");
        assert_eq!(target_test.id, 5);
        assert_eq!(target_test.parent_id, 5);

        assert_eq!(
            tree.is_test_contains_value(target_test.clone(), "stub"),
            true
        );
        assert_eq!(
            tree.is_test_contains_value(target_test.clone(), "base_protocol"),
            true
        );
        assert_eq!(
            tree.is_test_contains_value(target_test.clone(), ".py"),
            true
        );

        assert_eq!(
            tree.is_test_contains_value(target_test.clone(), "Function"),
            false
        );
        assert_eq!(
            tree.is_test_contains_value(target_test.clone(), ".rs"),
            false
        );
    }
}
