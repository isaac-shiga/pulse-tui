use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, LineGauge, Padding, Paragraph, Wrap},
};

use crate::app::Input;

use super::format::clock;
use super::{ACCENT, BG, DIM, FAINT, LINE, RAISED, TEXT, WARN, bold, colored, dim};

pub(super) const LABEL_WIDTH: usize = 14;
pub(super) const INPUT_WIDTH: usize = 30;
/// Where [`field`] values start: the focus bar plus the label column.
pub(super) const VALUE_COLUMN: u16 = LABEL_WIDTH as u16 + 2;
const QUOTE_LIFETIME: i64 = 120;

/// A rounded, hairline panel with a quiet title, and a spinner while busy.
pub(super) fn panel<'a>(title: &'a str, busy: Option<Span<'a>>) -> Block<'a> {
    let mut spans = vec![
        Span::raw(" "),
        Span::styled(title, Style::new().fg(DIM)),
        Span::raw(" "),
    ];
    spans.extend(busy);
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(LINE))
        .title(Line::from(spans))
        .padding(Padding::new(2, 2, 1, 0))
}

/// A bold title over a dim subtitle.
pub(super) fn heading<'a>(title: impl Into<String>, subtitle: impl Into<String>) -> Vec<Line<'a>> {
    vec![
        Line::from(Span::styled(title.into(), Style::new().fg(TEXT).bold())),
        Line::from(dim(subtitle.into())),
    ]
}

/// A form row: a focus bar, a label column, the value, then a hint.
pub(super) fn field<'a>(
    label: &str,
    focused: bool,
    value: Vec<Span<'a>>,
    hint: Option<Span<'a>>,
) -> Line<'a> {
    let label_style = if focused {
        Style::new().fg(TEXT).bold()
    } else {
        Style::new().fg(DIM)
    };
    let mut spans = vec![
        colored(if focused { "▎ " } else { "  " }, ACCENT),
        Span::styled(format!("{label:<LABEL_WIDTH$}"), label_style),
    ];
    spans.extend(value);
    if let Some(hint) = hint {
        spans.push(Span::raw("  "));
        spans.push(hint);
    }
    Line::from(spans)
}

/// Indents a line to the value column of [`field`].
pub(super) fn under_field(mut spans: Vec<Span<'_>>) -> Line<'_> {
    spans.insert(0, Span::raw(" ".repeat(VALUE_COLUMN as usize)));
    Line::from(spans)
}

/// A text box for an [`Input`].
pub(super) fn input(
    field: &Input,
    focused: bool,
    placeholder: &'static str,
    width: usize,
) -> Vec<Span<'static>> {
    let cursor = if field.secret {
        field.display().chars().count()
    } else {
        field.cursor()
    };
    text_box(field.display(), cursor, focused, placeholder, width)
}

/// A fixed-width text box. The focused box is lit and shows a block cursor
/// at `cursor`, a character index into `text`. Text wider than the box
/// scrolls to keep the cursor in view.
pub(super) fn text_box(
    text: String,
    cursor: usize,
    focused: bool,
    placeholder: &'static str,
    width: usize,
) -> Vec<Span<'static>> {
    let bg = if focused {
        RAISED
    } else {
        Color::Rgb(19, 21, 26)
    };
    let plain = Style::new().fg(TEXT).bg(bg);
    let caret = Style::new().fg(BG).bg(ACCENT);
    let room = width.saturating_sub(2).max(1);
    let mut spans = vec![Span::styled(" ", Style::new().bg(bg))];
    let used;
    if text.is_empty() {
        if focused {
            spans.push(Span::styled(" ", caret));
        }
        let hint: String = placeholder.chars().take(room.saturating_sub(1)).collect();
        used = hint.chars().count() + usize::from(focused);
        spans.push(Span::styled(hint, Style::new().fg(FAINT).bg(bg)));
    } else {
        let chars: Vec<char> = text.chars().collect();
        let cursor = cursor.min(chars.len());
        // Keep one cell for a cursor at the end of the text.
        let start = (cursor + 1).saturating_sub(room);
        let end = (start + room).min(chars.len());
        let style = if focused { plain.bold() } else { plain };
        let piece = |from: usize, to: usize| chars[from..to].iter().collect::<String>();
        if focused {
            spans.push(Span::styled(piece(start, cursor), style));
            let under = chars.get(cursor).map_or(" ".to_string(), char::to_string);
            spans.push(Span::styled(under, caret));
            if cursor + 1 < end {
                spans.push(Span::styled(piece(cursor + 1, end), style));
            }
            used = end.max(cursor + 1) - start;
        } else {
            spans.push(Span::styled(piece(start, end), style));
            used = end - start;
        }
    }
    spans.push(Span::styled(
        " ".repeat(room.saturating_sub(used) + 1),
        Style::new().bg(bg),
    ));
    spans
}

/// A value the user cycles with the arrow keys.
pub(super) fn selector(value: &str, focused: bool) -> Vec<Span<'static>> {
    let arrow = if focused { ACCENT } else { FAINT };
    let value = if focused {
        bold(value.to_string())
    } else {
        Span::styled(value.to_string(), Style::new().fg(TEXT))
    };
    vec![colored("‹ ", arrow), value, colored(" ›", arrow)]
}

/// A row of options with the chosen one lit.
pub(super) fn segmented(options: &[&str], chosen: usize, focused: bool) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    for (i, option) in options.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" "));
        }
        let style = match (i == chosen, focused) {
            (true, true) => Style::new().fg(BG).bg(ACCENT).bold(),
            (true, false) => Style::new().fg(TEXT).bg(RAISED).bold(),
            (false, _) => Style::new().fg(DIM),
        };
        spans.push(Span::styled(format!(" {option} "), style));
    }
    spans
}

/// A key drawn as a small cap, for inline actions.
pub(super) fn keycap(key: &'static str) -> Span<'static> {
    Span::styled(format!(" {key} "), Style::new().fg(TEXT).bg(RAISED).bold())
}

/// One row of a [`Ledger`].
pub(super) enum Row<'a> {
    /// A label on the left and a value flush right. When both do not fit
    /// on one row, the value moves under the label.
    Pair(Span<'a>, Line<'a>),
    /// A line that wraps to the width.
    Text(Line<'a>),
    /// A dotted divider.
    Divider,
    Gap,
}

impl<'a> Row<'a> {
    pub(super) fn pair(label: &'a str, value: impl Into<Line<'a>>) -> Self {
        Row::Pair(dim(label), value.into())
    }

    pub(super) fn text(line: impl Into<Line<'a>>) -> Self {
        Row::Text(line.into())
    }

    fn fits(label: &Span, value: &Line, width: u16) -> bool {
        label.width() + 2 + value.width() <= width as usize
    }

    fn height(&self, width: u16) -> u16 {
        match self {
            Row::Pair(label, value) if !Self::fits(label, value, width) => 2,
            Row::Text(line) => Paragraph::new(line.clone())
                .wrap(Wrap { trim: false })
                .line_count(width) as u16,
            _ => 1,
        }
    }
}

/// Rows of labels and values, like a receipt.
pub(super) struct Ledger<'a>(pub Vec<Row<'a>>);

impl Ledger<'_> {
    /// The rows the ledger needs at this width.
    pub(super) fn height(&self, width: u16) -> u16 {
        self.0.iter().map(|row| row.height(width)).sum()
    }
}

impl Widget for Ledger<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut y = area.y;
        for row in self.0 {
            let height = row.height(area.width).min(area.bottom().saturating_sub(y));
            if height == 0 {
                break;
            }
            let rect = Rect::new(area.x, y, area.width, height);
            y += height;
            match row {
                Row::Pair(label, value) if Row::fits(&label, &value, rect.width) => {
                    let [left, right] = Layout::horizontal([
                        Constraint::Length(label.width() as u16),
                        Constraint::Fill(1),
                    ])
                    .spacing(2)
                    .areas(rect);
                    label.render(left, buf);
                    value.right_aligned().render(right, buf);
                }
                Row::Pair(label, value) => {
                    Paragraph::new(vec![Line::from(label), value]).render(rect, buf);
                }
                Row::Text(line) => {
                    Paragraph::new(line)
                        .wrap(Wrap { trim: false })
                        .render(rect, buf);
                }
                Row::Divider => Block::new()
                    .borders(Borders::TOP)
                    .border_type(BorderType::LightDoubleDashed)
                    .border_style(Style::new().fg(FAINT))
                    .render(rect, buf),
                Row::Gap => {}
            }
        }
    }
}

/// A thin gauge that drains as a quote ages, labelled with the time left.
pub(super) struct Countdown<'a> {
    pub secs: i64,
    pub spinner: Span<'a>,
}

impl Widget for Countdown<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let s = self.secs.clamp(0, QUOTE_LIFETIME);
        let (color, label) = match s {
            0 => (FAINT, Line::from(vec![self.spinner, dim("new price ")])),
            1..=30 => (
                WARN,
                Line::from(colored(format!("held {} ", clock(s)), WARN)),
            ),
            _ => (
                ACCENT,
                Line::from(colored(format!("held {} ", clock(s)), ACCENT)),
            ),
        };
        LineGauge::default()
            .ratio(s as f64 / QUOTE_LIFETIME as f64)
            .label(label)
            .filled_symbol("━")
            .unfilled_symbol("─")
            .filled_style(Style::new().fg(color))
            .unfilled_style(Style::new().fg(LINE))
            .render(area, buf);
    }
}
