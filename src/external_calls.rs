use crate::entities::ParsedTest;
use anyhow::bail;
use anyhow::Result;
use std::env;
use std::process::Command;
use std::process::Output;
use std::process::Stdio;

#[cfg(target_os = "linux")]
pub fn run_command_in_shell(command: &str) -> Result<()> {
    if let Err(_) = Command::new("gnome-terminal")
        .arg("--version")
        .stdout(Stdio::null())
        .status()
    {
        bail!("Not implemented for your terminal")
    }
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

#[cfg(target_os = "macos")]
pub fn run_command_in_shell(command: &str) -> Result<()> {
    // we need to restore venv, cause we lost it after new terminal creation
    let pwd = env::var("PWD")?;
    let venv = env::var("VIRTUAL_ENV")?;
    let wrapped_command =
        format!("tell application \"Terminal\" to do script \"cd {pwd} && {venv}/bin/{command}\"");
    let output = Command::new("osascript")
        .arg("-e")
        .arg(wrapped_command)
        .output()?;
    match output.stderr.is_empty() {
        true => Ok(()),
        false => {
            let error: String = String::from_utf8_lossy(&output.stderr).try_into()?;
            bail!(error)
        }
    }
}

#[cfg(target_os = "windows")]
pub fn run_command_in_shell(command: &str) -> Result<()> {
    bail!("Not implemented for your os")
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

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "macos")]
pub fn open_editor(test: &ParsedTest) -> Result<()> {
    let file = test.full_path.split("::").next().unwrap();
    Command::new("open").arg("-t").arg(file).output()?;
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn open_editor(test: &ParsedTest) -> Result<()> {
    bail!("Not implemented for your os")
}
