use crate::entities::ParsedTest;
use anyhow::bail;
use anyhow::Result;
use std::env;
use std::process::Command;
use std::process::Output;
use std::process::Stdio;

fn open_in_gnome_terminal(command: &str) -> Result<()> {
    let shell = env::var("SHELL")?;
    let output = Command::new("gnome-terminal")
        .arg("--title=newWindow")
        .arg("--")
        .arg(format!("{}", shell))
        .arg("-c")
        .arg(command)
        .output()?;
    match output.stderr.is_empty() {
        true => Ok(()),
        false => {
            let error: String = String::from_utf8_lossy(&output.stderr).try_into()?;
            bail!(error)
        }
    }
}

pub fn run_command_in_shell(command: &str) -> Result<()> {
    match Command::new("gnome-terminal")
        .arg("--version")
        .stdout(Stdio::null())
        .status()
    {
        Ok(_) => open_in_gnome_terminal(command),
        Err(_) => bail!("Not implemented"),
    }
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

pub fn open_editor(test: &ParsedTest) -> Result<()> {
    let file = test.full_path.split("::").next().unwrap();
    let editor = env::var("EDITOR")?;
    let command: String;
    if editor.as_str().contains("hx") {
        command = format!("${} {}:{}", "EDITOR", file, test.row_location)
    } else if editor.as_str().contains("vi") {
        command = format!("${} {} +{}", "EDITOR", file, test.row_location)
    } else if editor.as_str().contains("nano") {
        command = format!("${} +{} {}", "EDITOR", test.row_location, file)
    } else if editor.as_str().contains("code") {
        command = format!("${} -g {}:{}", "EDITOR", file, test.row_location)
    } else if editor.as_str().contains("pycharm") {
        command = format!("${} -line {} {}", "EDITOR", test.row_location, file)
    } else {
        command = format!("${} {}", "EDITOR", file)
    };
    run_command_in_shell(&command)?;
    Ok(())
}
