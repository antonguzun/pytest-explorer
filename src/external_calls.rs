use std::process::{Command, Output};

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
