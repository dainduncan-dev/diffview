use crate::app::App;
use crate::render::{RowKind, SideRow};
use crate::theme::Theme;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let size = frame.area();
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(size);

    let top = vertical[0];
    let body = vertical[1];
    let footer = vertical[2];

    let title = if app.is_empty() {
        "diffview".to_string()
    } else {
        format!("{}/{}", app.file_index + 1, app.file_count())
    };
    let theme = app.theme.clone();
    let file_name = if app.is_empty() {
        "".to_string()
    } else {
        app.current_file_name()
    };
    render_header(frame, &title, &file_name, &theme, top);

    if app.is_empty() {
        let empty = Paragraph::new("No diff to display")
            .style(Style::default().fg(theme.dim_fg))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.border_left))
                    .border_type(BorderType::Rounded),
            );
        frame.render_widget(empty, body);
        render_footer(frame, &theme, footer, app.show_untracked);
        return;
    }

    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(body);

    let left_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_left))
        .border_type(BorderType::Rounded)
        .title("Before");
    let right_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_right))
        .border_type(BorderType::Rounded)
        .title("After");

    let left_inner = left_block.inner(panes[0]);
    let right_inner = right_block.inner(panes[1]);

    let (left_digits, right_digits) = app.line_digits();
    let scroll = app.scroll;
    let left_gutter = left_digits + 1;
    let right_gutter = right_digits + 1;

    let left_content_width = left_inner.width.saturating_sub(left_gutter as u16) as usize;
    let right_content_width = right_inner.width.saturating_sub(right_gutter as u16) as usize;

    let view = app
        .view(left_content_width, right_content_width)
        .expect("view");
    let height = left_inner.height.min(right_inner.height) as usize;
    let start = scroll.min(view.total_rows);
    let end = (start + height).min(view.total_rows);

    let left_lines = build_lines(
        slice_rows(&view.left_rows, start, end),
        left_digits,
        left_gutter,
        &theme,
    );
    let right_lines = build_lines(
        slice_rows(&view.right_rows, start, end),
        right_digits,
        right_gutter,
        &theme,
    );

    let left = Paragraph::new(left_lines)
        .block(left_block)
        .style(Style::default().fg(theme.base_fg));
    let right = Paragraph::new(right_lines)
        .block(right_block)
        .style(Style::default().fg(theme.base_fg));

    frame.render_widget(left, panes[0]);
    frame.render_widget(right, panes[1]);

    render_footer(frame, &theme, footer, app.show_untracked);
}

fn render_header(frame: &mut Frame, title: &str, file_name: &str, theme: &Theme, area: Rect) {
    let header = if file_name.is_empty() {
        Paragraph::new(title).style(
            Style::default()
                .fg(theme.header_chip_fg)
                .bg(theme.header_chip_bg),
        )
    } else {
        let line = Line::from(vec![
            Span::styled(
                format!(" {title} "),
                Style::default()
                    .fg(theme.border_left)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                file_name.to_string(),
                Style::default().fg(theme.header_chip_fg),
            ),
        ]);
        Paragraph::new(line).style(
            Style::default()
                .fg(theme.header_chip_fg)
                .bg(theme.header_chip_bg),
        )
    };
    frame.render_widget(header, area);
}

fn render_footer(frame: &mut Frame, theme: &Theme, area: Rect, show_untracked: bool) {
    let base = Style::default().fg(theme.footer_fg);
    let mut spans = vec![
        Span::styled("1-9 jump", base),
        Span::styled("  ", base),
        Span::styled("g/G top/bottom", base),
        Span::styled("  ", base),
        Span::styled("ctrl+u/d page", base),
        Span::styled("  ", base),
        Span::styled("f/b file", base),
        Span::styled("  ", base),
        Span::styled("n/p hunk", base),
        Span::styled("  ", base),
        Span::styled("u untracked", base),
    ];
    if show_untracked {
        spans.push(Span::styled(" [on]", Style::default().fg(theme.warn_fg)));
    }
    spans.push(Span::styled("  ", base));
    spans.push(Span::styled("q quit", base));
    let footer = Paragraph::new(Line::from(spans));
    frame.render_widget(footer, area);
}

fn build_lines(
    rows: &[SideRow],
    digits: usize,
    gutter_width: usize,
    theme: &Theme,
) -> Vec<Line<'static>> {
    rows.iter()
        .map(|row| {
            let number = match row.line {
                Some(value) => format!("{:>width$}", value, width = digits),
                None => " ".repeat(digits),
            };
            let mut content = String::with_capacity(gutter_width + row.text.len());
            content.push_str(&number);
            content.push(' ');
            content.push_str(&row.text);

            let style = style_row(row.kind, theme);
            Line::styled(content, style)
        })
        .collect()
}

fn slice_rows(rows: &[SideRow], start: usize, end: usize) -> &[SideRow] {
    if start >= rows.len() {
        return &[];
    }
    let clamped_end = end.min(rows.len());
    &rows[start..clamped_end]
}

fn style_row(kind: RowKind, theme: &Theme) -> Style {
    match kind {
        RowKind::Add => Style::default().fg(theme.add_fg).bg(theme.add_bg),
        RowKind::Del => Style::default().fg(theme.del_fg).bg(theme.del_bg),
        RowKind::NoNewline => Style::default().fg(theme.warn_fg).bg(theme.warn_bg),
        RowKind::Binary => Style::default().fg(theme.meta_fg),
        RowKind::Context => Style::default().fg(theme.base_fg),
    }
}
