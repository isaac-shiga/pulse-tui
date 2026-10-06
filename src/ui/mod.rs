use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};
use throbber_widgets_tui::{BRAILLE_EIGHT, Throbber};

mod components;
mod flow;
pub(crate) mod format;
mod home;
mod orders;
mod settings;

use crate::api::Env;
use crate::app::{App, Screen, ToastKind};

pub(super) const BG: Color = Color::Rgb(12, 13, 16);
pub(super) const RAISED: Color = Color::Rgb(27, 30, 37);
pub(super) const LINE: Color = Color::Rgb(44, 48, 57);
pub(super) const TEXT: Color = Color::Rgb(229, 231, 235);
pub(super) const DIM: Color = Color::Rgb(139, 145, 156);
pub(super) const FAINT: Color = Color::Rgb(82, 88, 99);
pub(super) const ACCENT: Color = Color::Rgb(94, 234, 212);
pub(super) const OK: Color = Color::Rgb(163, 230, 53);
pub(super) const WARN: Color = Color::Rgb(251, 191, 36);
pub(super) const ERR: Color = Color::Rgb(251, 113, 133);
const MAX_WIDTH: u16 = 100;

pub(super) type Hints = Vec<(&'static str, &'static str)>;

pub(super) fn dim<'a>(s: impl Into<std::borrow::Cow<'a, str>>) -> Span<'a> {
    Span::styled(s, Style::new().fg(DIM))
}

pub(super) fn faint<'a>(s: impl Into<std::borrow::Cow<'a, str>>) -> Span<'a> {
    Span::styled(s, Style::new().fg(FAINT))
}

pub(super) fn bold<'a>(s: impl Into<std::borrow::Cow<'a, str>>) -> Span<'a> {
    Span::styled(s, Style::new().fg(TEXT).bold())
}

pub(super) fn colored<'a>(s: impl Into<std::borrow::Cow<'a, str>>, color: Color) -> Span<'a> {
    Span::styled(s, Style::new().fg(color))
}

/// The busy spinner, with one space after it.
pub(super) fn spinner(app: &App) -> Span<'static> {
    Throbber::default()
        .throbber_set(BRAILLE_EIGHT)
        .throbber_style(Style::new().fg(ACCENT))
        .to_symbol_span(&app.throbber)
}

/// Blends two colors. `t` runs from 0 (all `a`) to 1 (all `b`).
pub(super) fn mix(a: Color, b: Color, t: f32) -> Color {
    match (a, b) {
        (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => {
            let t = t.clamp(0.0, 1.0);
            let lerp = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
            Color::Rgb(lerp(r1, r2), lerp(g1, g2), lerp(b1, b2))
        }
        _ => b,
    }
}

pub fn render(f: &mut Frame, app: &App) {
    f.render_widget(Block::new().style(Style::new().bg(BG).fg(TEXT)), f.area());
    let [head, body, foot] = Layout::vertical([
        Constraint::Length(LOGO.len() as u16 + 1),
        Constraint::Fill(1),
        Constraint::Length(2),
    ])
    .areas(f.area());
    header(f, head, app);
    let width = body.width.saturating_sub(4).min(MAX_WIDTH);
    let area = Rect {
        x: body.x + (body.width - width) / 2,
        y: body.y + 1,
        width,
        height: body.height.saturating_sub(1),
    };
    let hints = match app.screen {
        Screen::Home => home::render(f, area, app),
        Screen::Flow => flow::render(f, area, app),
        Screen::Orders => orders::render(f, area, app),
        Screen::Settings => settings::render(f, area, app),
    };
    footer(f, foot, app, &hints);
}

/// The Pulse logo, hand-drawn in quadrant blocks for a three-row header.
/// The first six columns are the mark.
const LOGO: [&str; 3] = [
    "▄▟███▙  █▀▙     █        ",
    "█▌ ▄▟█  █▄▛ █ █ █ ▟█▀ ▟█▙",
    "███▀▀   █   ▜▄█ █ ▄█▛ ▜▄▄",
];
const MARK_COLS: usize = 6;

fn header(f: &mut Frame, area: Rect, app: &App) {
    // Live mode tints the rule red, so real money is never ambiguous.
    let rule = match app.env() {
        Env::Test => LINE,
        Env::Live => Color::Rgb(136, 19, 55),
    };
    let block = Block::new()
        .borders(Borders::BOTTOM)
        .border_style(Style::new().fg(rule));
    let inner = block.inner(area).inner(Margin::new(2, 0));
    f.render_widget(block, area);
    // Badges and the breadcrumb sit on the middle row, level with the logo.
    let middle = Rect {
        y: inner.y + inner.height / 2,
        height: 1,
        ..inner
    };
    // Home shows the large logo, so the header leaves it out there.
    if app.screen != Screen::Home {
        let logo: Vec<Line> = LOGO
            .iter()
            .map(|row| {
                let mark: String = row.chars().take(MARK_COLS).collect();
                let word: String = row.chars().skip(MARK_COLS).collect();
                Line::from(vec![colored(mark, ACCENT), bold(word)])
            })
            .collect();
        f.render_widget(Paragraph::new(logo), inner);
        let crumb = match app.screen {
            Screen::Flow => app.flow.dir.title(),
            Screen::Orders => "Orders",
            _ => "Settings",
        };
        let after_logo = LOGO[0].chars().count() as u16;
        f.render_widget(
            Line::from(vec![faint(" /  "), dim(crumb)]),
            Rect {
                x: middle.x + after_logo,
                width: middle.width.saturating_sub(after_logo),
                ..middle
            },
        );
    }

    let key = if app.key().is_empty() {
        colored("no key   ", ERR)
    } else {
        faint("key set   ")
    };
    let badge = match app.env() {
        Env::Test => vec![colored("● ", WARN), colored("test mode", WARN)],
        Env::Live => vec![Span::styled(
            " ● LIVE ",
            Style::new()
                .fg(Color::Rgb(255, 255, 255))
                .bg(Color::Rgb(190, 18, 60))
                .bold(),
        )],
    };
    let mut right = vec![key];
    right.extend(badge);
    f.render_widget(Line::from(right).right_aligned(), middle);
}

fn footer(f: &mut Frame, area: Rect, app: &App, hints: &Hints) {
    let block = Block::new()
        .borders(Borders::TOP)
        .border_style(Style::new().fg(LINE));
    let line = block.inner(area).inner(Margin::new(2, 0));
    f.render_widget(block, area);
    if let Some(t) = &app.toast {
        let (icon, color) = match t.kind {
            ToastKind::Info => ("●", ACCENT),
            ToastKind::Success => ("✓", OK),
            ToastKind::Error => ("✕", ERR),
        };
        let toast = Line::from(vec![
            colored(format!("{icon}  "), color),
            bold(t.text.clone()),
        ]);
        f.render_widget(Paragraph::new(toast), line);
        return;
    }
    let mut spans = Vec::new();
    for (i, (key, label)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("    "));
        }
        spans.push(Span::styled(*key, Style::new().fg(TEXT).bold()));
        spans.push(dim(format!(" {label}")));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), line);
}
