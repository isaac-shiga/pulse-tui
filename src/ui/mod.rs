use ratatui::{prelude::*, widgets::Paragraph};

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
const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
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

pub(super) fn spinner(app: &App) -> &'static str {
    SPINNER[app.frame % SPINNER.len()]
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
    f.render_widget(
        ratatui::widgets::Block::new().style(Style::new().bg(BG).fg(TEXT)),
        f.area(),
    );
    let [head, body, foot] = Layout::vertical([
        Constraint::Length(2),
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

fn header(f: &mut Frame, area: Rect, app: &App) {
    let [line, rule] = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(area);
    let line = line.inner(Margin::new(2, 0));
    // The dot beats twice, then rests, like a pulse.
    let beat = matches!(app.frame % 12, 0 | 1 | 3 | 4);
    let dot = if beat { ACCENT } else { mix(ACCENT, BG, 0.6) };
    let crumb = match app.screen {
        Screen::Home => "",
        Screen::Flow => app.flow.dir.title(),
        Screen::Orders => "Orders",
        Screen::Settings => "Settings",
    };
    let mut left = vec![colored("● ", dot), bold("pulse")];
    if !crumb.is_empty() {
        left.push(faint("  /  "));
        left.push(dim(crumb));
    }
    f.render_widget(Paragraph::new(Line::from(left)), line);

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
    f.render_widget(
        Paragraph::new(Line::from(right)).alignment(Alignment::Right),
        line,
    );
    // Live mode tints the rule red, so real money is never ambiguous.
    let color = match app.env() {
        Env::Test => LINE,
        Env::Live => Color::Rgb(136, 19, 55),
    };
    f.render_widget(
        Paragraph::new(colored("─".repeat(rule.width as usize), color)),
        rule,
    );
}

fn footer(f: &mut Frame, area: Rect, app: &App, hints: &Hints) {
    let [rule, line] = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(area);
    f.render_widget(
        Paragraph::new(colored("─".repeat(rule.width as usize), LINE)),
        rule,
    );
    let line = line.inner(Margin::new(2, 0));
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
