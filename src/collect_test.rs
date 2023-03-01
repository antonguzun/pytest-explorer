use regex::Regex;
use std::process::Command;
use std::rc::Rc;

#[derive(Debug, Clone)]
enum PytestElementType {
    Module,
    Class,
    Function,
}

#[derive(Debug, Clone)]
pub struct PytestElement {
    pub id: usize,
    pub name: String,
    parent: Option<Rc<PytestElement>>,
    element_type: PytestElementType,
}

#[derive(Debug, Clone)]
pub struct PytestTree {
    id: usize,
    pub elements: Vec<Rc<PytestElement>>,
}

impl PytestElement {
    pub fn full_test_name(&self) -> String {
        match self.element_type {
            PytestElementType::Module => return self.name.clone(),
            PytestElementType::Class => {
                return format!("{}::{}", self.parent.clone().unwrap().name, self.name);
            }
            PytestElementType::Function => {
                let parent = self.parent.clone().unwrap();
                match &parent.parent {
                    Some(module) => format!("{}::{}::{}", module.name, parent.name, self.name),
                    None => format!("{}::{}", parent.name, self.name),
                }
            }
        }
    }
    pub fn is_test_contains_value(&self, value: &str) -> bool {
        let mut targets = vec![&self.name];
        match &self.parent {
            Some(p) => {
                targets.push(&p.name);
                match &p.parent {
                    Some(pp) => {
                        targets.push(&pp.name);
                    }
                    None => {}
                }
            }
            None => {}
        };
        targets.iter().any(|t| t.contains(value))
    }
}
impl PytestTree {
    pub fn new(id: usize, elements: Vec<Rc<PytestElement>>) -> Self {
        Self { id, elements }
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
    let mut elements = vec![];
    let tree_id = counter.get();
    let re = Regex::new(r"^(\s*)<(Function|Class|Module)\s(.*)>$").unwrap();
    let mut last_module: Option<Rc<PytestElement>> = None;
    let mut last_class_in_module: Option<Rc<PytestElement>> = None;
    for line in input.split("\n") {
        match re.captures(line) {
            Some(caps) => {
                let ident: usize = caps.get(1).map_or("", |m| m.as_str()).len();
                let name = caps.get(3).unwrap().as_str().to_string();
                match caps.get(2).unwrap().as_str() {
                    "Function" => {
                        let parent = match &last_class_in_module {
                            Some(p_c) => Some(Rc::clone(p_c)),
                            None => match &last_module {
                                Some(p_m) => Some(Rc::clone(p_m)),
                                None => panic!("Wrong test input, Function have to own parent"),
                            },
                        };
                        let e = PytestElement {
                            id: counter.get(),
                            name,
                            parent,
                            element_type: PytestElementType::Function,
                        };
                        let element = Rc::new(e);
                        elements.push(element);
                    }
                    "Class" => {
                        let parent = match &last_module {
                            Some(p) => Some(Rc::clone(p)),
                            None => None,
                        };
                        let e = PytestElement {
                            id: counter.get(),
                            name,
                            parent,
                            element_type: PytestElementType::Class,
                        };
                        let element = Rc::new(e);
                        last_class_in_module = Some(Rc::clone(&element));
                        elements.push(element);
                    }
                    "Module" => {
                        let e = PytestElement {
                            id: counter.get(),
                            name,
                            parent: None,
                            element_type: PytestElementType::Module,
                        };
                        let element = Rc::new(e);
                        last_module = Some(Rc::clone(&element));
                        last_class_in_module = None;
                        elements.push(element);
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
        assert_eq!(target_test.parent.clone().unwrap().id, 1);

        assert_eq!(target_test.is_test_contains_value("stub"), true);
        assert_eq!(target_test.is_test_contains_value("base_protocol"), true);
        assert_eq!(target_test.is_test_contains_value(".py"), true);

        assert_eq!(target_test.is_test_contains_value("Function"), false);
        assert_eq!(target_test.is_test_contains_value(".rs"), false);
    }
}
