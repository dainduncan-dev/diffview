use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Diff {
    pub files: Vec<FileDiff>,
}

#[derive(Debug, Clone)]
pub struct FileDiff {
    pub old_path: String,
    pub new_path: String,
    pub hunks: Vec<Hunk>,
    pub is_binary: bool,
    pub max_old_line: u32,
    pub max_new_line: u32,
}

#[derive(Debug, Clone)]
pub struct Hunk {
    pub old_start: u32,
    pub new_start: u32,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub old_line: Option<u32>,
    pub new_line: Option<u32>,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind {
    Context,
    Add,
    Del,
    NoNewline,
}

pub fn parse_diff(input: &str) -> Result<Diff> {
    let mut diff = Diff { files: Vec::new() };
    let mut current_file: Option<FileDiff> = None;
    let mut current_hunk: Option<Hunk> = None;
    let mut old_line_no: u32 = 0;
    let mut new_line_no: u32 = 0;

    for raw_line in input.lines() {
        let line = raw_line;

        if line.starts_with("diff --git ") {
            flush_hunk(&mut current_file, &mut current_hunk);
            if let Some(file) = current_file.take() {
                diff.files.push(file);
            }
            let (old_path, new_path) = parse_diff_header(line)?;
            current_file = Some(FileDiff {
                old_path,
                new_path,
                hunks: Vec::new(),
                is_binary: false,
                max_old_line: 0,
                max_new_line: 0,
            });
            continue;
        }

        if line.starts_with("Binary files ") || line.starts_with("GIT binary patch") {
            if let Some(file) = current_file.as_mut() {
                file.is_binary = true;
            }
            continue;
        }

        if line.starts_with("--- ") {
            if let Some(file) = current_file.as_mut() {
                file.old_path = parse_file_path(line.trim_start_matches("--- "));
            }
            continue;
        }

        if line.starts_with("+++ ") {
            if let Some(file) = current_file.as_mut() {
                file.new_path = parse_file_path(line.trim_start_matches("+++ "));
            }
            continue;
        }

        if line.starts_with("@@") {
            flush_hunk(&mut current_file, &mut current_hunk);
            let hunk = parse_hunk_header(line).context("failed to parse hunk header")?;
            old_line_no = hunk.old_start;
            new_line_no = hunk.new_start;
            current_hunk = Some(hunk);
            continue;
        }

        if let Some(hunk) = current_hunk.as_mut() {
            if line == "\\ No newline at end of file" {
                hunk.lines.push(DiffLine {
                    kind: DiffLineKind::NoNewline,
                    old_line: None,
                    new_line: None,
                    text: "No newline at end of file".to_string(),
                });
                continue;
            }

            if let Some(rest) = line.strip_prefix(' ') {
                hunk.lines.push(DiffLine {
                    kind: DiffLineKind::Context,
                    old_line: Some(old_line_no),
                    new_line: Some(new_line_no),
                    text: rest.to_string(),
                });
                old_line_no += 1;
                new_line_no += 1;
                update_max_lines(
                    current_file.as_mut(),
                    Some(old_line_no - 1),
                    Some(new_line_no - 1),
                );
                continue;
            }

            if let Some(rest) = line.strip_prefix('+') {
                hunk.lines.push(DiffLine {
                    kind: DiffLineKind::Add,
                    old_line: None,
                    new_line: Some(new_line_no),
                    text: rest.to_string(),
                });
                update_max_lines(current_file.as_mut(), None, Some(new_line_no));
                new_line_no += 1;
                continue;
            }

            if let Some(rest) = line.strip_prefix('-') {
                hunk.lines.push(DiffLine {
                    kind: DiffLineKind::Del,
                    old_line: Some(old_line_no),
                    new_line: None,
                    text: rest.to_string(),
                });
                update_max_lines(current_file.as_mut(), Some(old_line_no), None);
                old_line_no += 1;
                continue;
            }
        }
    }

    flush_hunk(&mut current_file, &mut current_hunk);
    if let Some(file) = current_file.take() {
        diff.files.push(file);
    }

    Ok(diff)
}

fn flush_hunk(current_file: &mut Option<FileDiff>, current_hunk: &mut Option<Hunk>) {
    if let Some(hunk) = current_hunk.take() {
        if let Some(file) = current_file.as_mut() {
            file.hunks.push(hunk);
        }
    }
}

fn parse_diff_header(line: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    let old_path = parts.get(2).copied().unwrap_or("");
    let new_path = parts.get(3).copied().unwrap_or("");
    Ok((strip_prefixes(old_path), strip_prefixes(new_path)))
}

fn parse_file_path(raw: &str) -> String {
    strip_prefixes(raw.trim())
}

fn strip_prefixes(path: &str) -> String {
    path.trim_start_matches("a/")
        .trim_start_matches("b/")
        .to_string()
}

fn parse_hunk_header(line: &str) -> Result<Hunk> {
    let trimmed = line.trim_matches('@').trim();
    let mut parts = trimmed.split_whitespace();
    let old_part = parts.next().context("missing old hunk range")?;
    let new_part = parts.next().context("missing new hunk range")?;
    let (old_start, _) = parse_range(old_part.trim_start_matches('-'))?;
    let (new_start, _) = parse_range(new_part.trim_start_matches('+'))?;
    Ok(Hunk {
        old_start,
        new_start,
        lines: Vec::new(),
    })
}

fn parse_range(input: &str) -> Result<(u32, u32)> {
    let mut iter = input.split(',');
    let start = iter
        .next()
        .context("missing range start")?
        .parse::<u32>()
        .context("invalid range start")?;
    let len = match iter.next() {
        Some(value) => value.parse::<u32>().context("invalid range length")?,
        None => 1,
    };
    Ok((start, len))
}

fn update_max_lines(file: Option<&mut FileDiff>, old_line: Option<u32>, new_line: Option<u32>) {
    if let Some(file) = file {
        if let Some(old_line) = old_line {
            file.max_old_line = file.max_old_line.max(old_line);
        }
        if let Some(new_line) = new_line {
            file.max_new_line = file.max_new_line.max(new_line);
        }
    }
}
