use crate::diff::{DiffLineKind, FileDiff};
use std::borrow::Cow;
use unicode_width::UnicodeWidthChar;

const TAB_WIDTH: usize = 4;

#[derive(Debug, Clone)]
pub struct SideRow {
    pub line: Option<u32>,
    pub text: String,
    pub kind: RowKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    Context,
    Add,
    Del,
    NoNewline,
    Binary,
}

#[derive(Debug, Clone)]
pub struct FileView {
    pub left_rows: Vec<SideRow>,
    pub right_rows: Vec<SideRow>,
    pub hunk_starts: Vec<usize>,
    pub total_rows: usize,
}

pub fn build_rows(file: &FileDiff, left_width: usize, right_width: usize) -> FileView {
    if file.is_binary {
        return FileView {
            left_rows: vec![SideRow {
                line: None,
                text: "Binary file".to_string(),
                kind: RowKind::Binary,
            }],
            right_rows: vec![SideRow {
                line: None,
                text: "Binary file".to_string(),
                kind: RowKind::Binary,
            }],
            hunk_starts: vec![0],
            total_rows: 1,
        };
    }

    let mut left_rows = Vec::new();
    let mut right_rows = Vec::new();
    let mut hunk_starts = Vec::new();

    for hunk in &file.hunks {
        hunk_starts.push(left_rows.len().max(right_rows.len()));
        for line in &hunk.lines {
            let (left_text, right_text, kind) = match line.kind {
                DiffLineKind::Context => (
                    Some(line.text.as_str()),
                    Some(line.text.as_str()),
                    RowKind::Context,
                ),
                DiffLineKind::Add => (None, Some(line.text.as_str()), RowKind::Add),
                DiffLineKind::Del => (Some(line.text.as_str()), None, RowKind::Del),
                DiffLineKind::NoNewline => (
                    Some(line.text.as_str()),
                    Some(line.text.as_str()),
                    RowKind::NoNewline,
                ),
            };

            let left_segments = wrap_text(left_text, left_width);
            let right_segments = wrap_text(right_text, right_width);

            for (index, segment) in left_segments.iter().enumerate() {
                left_rows.push(SideRow {
                    line: if index == 0 { line.old_line } else { None },
                    text: segment.clone(),
                    kind,
                });
            }

            for (index, segment) in right_segments.iter().enumerate() {
                right_rows.push(SideRow {
                    line: if index == 0 { line.new_line } else { None },
                    text: segment.clone(),
                    kind,
                });
            }
        }
    }

    let total_rows = left_rows.len().max(right_rows.len());
    FileView {
        left_rows,
        right_rows,
        hunk_starts,
        total_rows,
    }
}

fn wrap_text(text: Option<&str>, width: usize) -> Vec<String> {
    let Some(text) = text else {
        return Vec::new();
    };
    if width == 0 {
        return vec![String::new()];
    }

    let expanded: Cow<'_, str> = if text.contains('\t') {
        Cow::Owned(expand_tabs(text, TAB_WIDTH))
    } else {
        Cow::Borrowed(text)
    };
    let (first, rest) = split_once(&expanded, width);
    if rest.is_empty() {
        return vec![first];
    }

    let mut segments = Vec::new();
    segments.push(first);
    segments.extend(split_all(&rest, width));
    segments
}

fn split_once(text: &str, width: usize) -> (String, String) {
    let mut current = String::new();
    let mut current_width = 0;
    let mut iter = text.chars();

    while let Some(ch) = iter.next() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if current_width + ch_width > width && !current.is_empty() {
            let rest: String = std::iter::once(ch).chain(iter).collect();
            return (current, rest);
        }
        current.push(ch);
        current_width += ch_width;
    }

    (current, String::new())
}

fn split_all(text: &str, width: usize) -> Vec<String> {
    let mut segments = Vec::new();
    let mut remaining = text.to_string();
    while !remaining.is_empty() {
        let (head, tail) = split_once(&remaining, width);
        segments.push(head);
        remaining = tail;
    }
    segments
}

fn expand_tabs(text: &str, tab_width: usize) -> String {
    let mut output = String::new();
    let mut column = 0;

    for ch in text.chars() {
        if ch == '\t' {
            let spaces = tab_width.saturating_sub(column % tab_width).max(1);
            output.extend(std::iter::repeat(' ').take(spaces));
            column += spaces;
            continue;
        }

        output.push(ch);
        column += UnicodeWidthChar::width(ch).unwrap_or(0).max(1);
    }

    output
}
