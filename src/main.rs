mod app;
mod cli;
mod diff;
mod input;
mod render;
mod theme;
mod ui;

use crate::app::App;
use crate::cli::Cli;
use crate::input::{load_diff, load_untracked_diff};
use anyhow::Result;
use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::widgets::{Block, Borders};
use ratatui::Terminal;
use std::io;
use std::time::Duration;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let diff_text = load_diff(&cli)?;
    let diff = diff::parse_diff(&diff_text)?;
    let mut app = App::new(diff);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;
    let _guard = TerminalGuard;

    loop {
        terminal.draw(|frame| ui::draw(frame, &mut app))?;

        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    let area: ratatui::layout::Rect = terminal.size()?.into();
                    if handle_key(&key, &cli, &mut app, area)? {
                        break;
                    }
                }
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    }

    Ok(())
}

fn handle_key(
    key: &KeyEvent,
    cli: &Cli,
    app: &mut App,
    size: ratatui::layout::Rect,
) -> Result<bool> {
    let (left_width, right_width, height, total_rows) = view_metrics(size, app);
    let page = height.max(1) as i32;

    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('u') => app.scroll_by(-page, height, total_rows),
            KeyCode::Char('d') => app.scroll_by(page, height, total_rows),
            _ => {}
        }
        return Ok(false);
    }

    match key.code {
        KeyCode::Char('q') => return Ok(true),
        KeyCode::Char('j') | KeyCode::Down => app.scroll_by(1, height, total_rows),
        KeyCode::Char('k') | KeyCode::Up => app.scroll_by(-1, height, total_rows),
        KeyCode::Char('g') => app.jump_to_start(),
        KeyCode::Char('G') => app.jump_to_end(height, total_rows),
        KeyCode::Char('n') => app.next_hunk(left_width, right_width),
        KeyCode::Char('p') => app.prev_hunk(left_width, right_width),
        KeyCode::Char('f') => app.next_file(),
        KeyCode::Char('b') => app.prev_file(),
        KeyCode::Char('u') => {
            if !app.has_untracked() {
                let untracked_text = load_untracked_diff(cli)?;
                let untracked = diff::parse_diff(&untracked_text)?;
                app.set_untracked(untracked);
            }
            app.toggle_untracked();
        }
        KeyCode::Char(ch) if ch.is_ascii_digit() && ch != '0' => {
            let index = ch.to_digit(10).unwrap_or(1) as usize - 1;
            app.jump_to_file(index);
        }
        _ => {}
    }

    Ok(false)
}

fn view_metrics(size: ratatui::layout::Rect, app: &mut App) -> (usize, usize, usize, usize) {
    if app.is_empty() {
        return (0, 0, 0, 0);
    }

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(size);
    let body = vertical[1];
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(body);

    let left_block = Block::default().borders(Borders::ALL);
    let right_block = Block::default().borders(Borders::ALL);
    let left_inner = left_block.inner(panes[0]);
    let right_inner = right_block.inner(panes[1]);

    let (left_digits, right_digits) = app.line_digits();
    let left_gutter = left_digits + 1;
    let right_gutter = right_digits + 1;
    let left_width = left_inner.width.saturating_sub(left_gutter as u16) as usize;
    let right_width = right_inner.width.saturating_sub(right_gutter as u16) as usize;
    let height = left_inner.height.min(right_inner.height) as usize;
    let total_rows = app
        .view(left_width, right_width)
        .map(|view| view.total_rows)
        .unwrap_or(0);

    (left_width, right_width, height, total_rows)
}

struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}
