use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Padding},
};

use crate::app::Input;

use super::{ACCENT, BG, DIM, FAINT, LINE, RAISED, TEXT, bold, colored, dim, faint};

const LABEL_WIDTH: usize = 14;
pub(super) const INPUT_WIDTH: usize = 30;

/// A rounded, hairline panel with a quiet title.
pub(super) fn panel(title: &str) -> Block<'_> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(LINE))
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled(title, Style::new().fg(DIM)),
            Span::raw(" "),
        ]))
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
    spans.insert(0, Span::raw(" ".repeat(LABEL_WIDTH + 2)));
    Line::from(spans)
}

/// A text box. The focused box is lit and shows the cursor.
pub(super) fn input(
    field: &Input,
    focused: bool,
    placeholder: &'static str,
    width: usize,
) -> Vec<Span<'static>> {
    let text = field.display();
    let bg = if focused {
        RAISED
    } else {
        Color::Rgb(19, 21, 26)
    };
    let (body, style) = if text.is_empty() {
        (placeholder.to_string(), Style::new().fg(FAINT).bg(bg))
    } else {
        (text, Style::new().fg(TEXT).bg(bg))
    };
    let mut spans = vec![Span::styled(" ", Style::new().bg(bg))];
    let used = body.chars().count() + 2;
    if focused && field.value.is_empty() {
        spans.push(Span::styled("▏", Style::new().fg(ACCENT).bg(bg)));
        spans.push(Span::styled(body, style));
    } else {
        spans.push(Span::styled(
            body,
            if focused { style.bold() } else { style },
        ));
        if focused {
            spans.push(Span::styled("▏", Style::new().fg(ACCENT).bg(bg)));
        } else {
            spans.push(Span::styled(" ", Style::new().bg(bg)));
        }
    }
    spans.push(Span::styled(
        " ".repeat(width.saturating_sub(used)),
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

/// A ledger row: the label on the left, the value flush right.
pub(super) fn pair<'a>(label: &'a str, value: Vec<Span<'a>>, width: u16) -> Line<'a> {
    pair_span(dim(label), value, width)
}

/// A ledger row with a styled label.
pub(super) fn pair_span<'a>(label: Span<'a>, value: Vec<Span<'a>>, width: u16) -> Line<'a> {
    let used = label.width() + value.iter().map(Span::width).sum::<usize>();
    let gap = (width as usize).saturating_sub(used).max(2);
    let mut spans = vec![label, Span::raw(" ".repeat(gap))];
    spans.extend(value);
    Line::from(spans)
}

/// A dotted divider for ledgers.
pub(super) fn divider(width: u16) -> Line<'static> {
    Line::from(faint("┄".repeat(width as usize)))
}
