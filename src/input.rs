use crate::cli::Cli;
use anyhow::{bail, Context, Result};
use std::fs;
use std::io::{self, Read};
use std::process::Command;

pub fn load_diff(cli: &Cli) -> Result<String> {
    if cli.stdin || (!atty::is(atty::Stream::Stdin) && cli.diff_file.is_none()) {
        let mut input = String::new();
        io::stdin()
            .read_to_string(&mut input)
            .context("failed to read diff from stdin")?;
        return Ok(input);
    }

    if let Some(path) = &cli.diff_file {
        return fs::read_to_string(path).context("failed to read diff file");
    }

    let (global_args, diff_args) = split_git_args(&cli.git_args);
    let mut cmd = Command::new("git");
    cmd.args(global_args)
        .arg("diff")
        .arg("--no-color")
        .arg("--unified=3")
        .arg("--no-prefix");
    for arg in diff_args {
        cmd.arg(arg);
    }

    let output = cmd.output().context("failed to run git diff")?;
    if !output.status.success() && output.status.code() != Some(1) {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git diff failed: {stderr}");
    }

    let stdout = String::from_utf8(output.stdout).context("git diff output was not valid UTF-8")?;
    Ok(stdout)
}

pub fn load_untracked_diff(cli: &Cli) -> Result<String> {
    if cli.stdin || cli.diff_file.is_some() {
        return Ok(String::new());
    }

    let (global_args, _) = split_git_args(&cli.git_args);
    let mut list_cmd = Command::new("git");
    list_cmd
        .args(&global_args)
        .arg("ls-files")
        .arg("--others")
        .arg("--exclude-standard");

    let output = list_cmd
        .output()
        .context("failed to list untracked files")?;
    if !output.status.success() {
        return Ok(String::new());
    }

    let stdout =
        String::from_utf8(output.stdout).context("git ls-files output was not valid UTF-8")?;
    let files: Vec<String> = stdout
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    if files.is_empty() {
        return Ok(String::new());
    }

    let mut combined = String::new();
    for path in files {
        let mut cmd = Command::new("git");
        cmd.args(&global_args)
            .arg("diff")
            .arg("--no-color")
            .arg("--unified=3")
            .arg("--no-prefix")
            .arg("--no-index")
            .arg("--")
            .arg("/dev/null")
            .arg(path);

        let output = cmd.output().context("failed to diff untracked file")?;
        if output.status.success() || output.status.code() == Some(1) {
            let chunk = String::from_utf8_lossy(&output.stdout);
            combined.push_str(&chunk);
            if !combined.ends_with('\n') {
                combined.push('\n');
            }
        }
    }

    Ok(combined)
}

fn split_git_args(args: &[String]) -> (Vec<String>, Vec<String>) {
    let mut global_args = Vec::new();
    let mut diff_args = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg == "-C" {
            if let Some(value) = args.get(i + 1) {
                global_args.push(arg.clone());
                global_args.push(value.clone());
                i += 2;
                continue;
            }
        }

        if arg.starts_with("--git-dir") || arg.starts_with("--work-tree") {
            global_args.push(arg.clone());
        } else {
            diff_args.push(arg.clone());
        }
        i += 1;
    }

    (global_args, diff_args)
}
