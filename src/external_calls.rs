use std::env;
use std::process::{Command, Output};
use anyhow::Result;
use crate::entities::ParsedTest;

pub fn run_command_in_shell(command: &str) {
    Command::new("gnome-terminal")
        .arg("--title=newWindow")
        .arg("--")
        .arg("zsh")
        .arg("-c")
        .arg(command)
        .spawn()
        .expect("run test in terminal command failed to start");
}

pub fn run_test(test_name: String) -> Output {
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

pub fn open_editor(test: &ParsedTest) -> Result<(), anyhow::Error> {
    let file = test.full_path.split("::").next().unwrap();
    let editor = env::var("EDITOR")?;
    let mut command: String = String::new();
    if editor.as_str().contains("hx") {
        command = format!("${} {}:{}", "EDITOR", file, test.row_location)
    } else if editor.as_str().contains("vi") {
        command = format!("${} {} +{}", "EDITOR", file, test.row_location)
    } else {
        command = format!("${} {}", "EDITOR", file)
    };
    run_command_in_shell(&command);
    Ok(())
}
