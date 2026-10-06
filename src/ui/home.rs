use ratatui::{
    prelude::*,
    symbols::Marker,
    widgets::{
        HighlightSpacing, List, ListItem, ListState, Paragraph,
        canvas::{Canvas, Line as Stroke},
    },
};

use crate::api::Env;
use crate::app::App;

use super::{
    ACCENT, BG, DIM, ERR, FAINT, Hints, RAISED, TEXT, WARN, bold, colored, dim, faint, mix,
};

/// The Pulse logo from pulseshiga.io, drawn in quadrant blocks. The
/// first 13 columns are the mark, the rest is the wordmark.
const LOGO: [&str; 5] = [
    "  ▗███████▙   ▄▄▄▄▄▄          ▗▄▖               ",
    "▟█▌     ▐██▌  ██▛▀▀▜█▖▗▄▖  ▗▄▖▐█▌ ▄▄▄▄▄   ▄▄▄▄▄ ",
    "██▌   ▄▄▟██▌  ██▌ ▄▟█▘▐█▌  ▐█▌▐█▌▐█▙▄▖▀▘ ▐█▙▄▟█▌",
    "██▙  ▐███▀▘   ██▛▀▀▀  ▐█▙ ▗▟█▌▐█▌▗▄▀▀▜█▌ ▐█▌  ▄▖",
    "█████▌        ▀▀▘      ▀▀▀▀▝▀▘▝▀▘ ▀▀▀▀▀   ▀▀▀▀▀ ",
];
const MARK_COLS: usize = 13;

const ITEMS: [(&str, &str); 4] = [
    ("Buy stablecoins", "Pay naira, receive USDT or USDC"),
    ("Sell stablecoins", "Send USDT or USDC, receive naira"),
    ("Orders", "Every order and where it stands"),
    ("Settings", "API keys and environment"),
];

const TRACE_ROWS: u16 = 3;
const FULL_HEIGHT: u16 = 27;

pub(super) fn render(f: &mut Frame, area: Rect, app: &App) -> Hints {
    let width = area.width.min(76);
    let area = Rect {
        x: area.x + (area.width - width) / 2,
        width,
        ..area
    };
    let tall = area.height >= FULL_HEIGHT && area.width >= 50;
    let pad = Margin::new(2, 0);
    if tall {
        let area = Rect {
            y: area.y + (area.height - FULL_HEIGHT) / 3,
            height: FULL_HEIGHT,
            ..area
        };
        let [mark, _, tagline, _, trace, _, menu, _, note] = Layout::vertical([
            Constraint::Length(LOGO.len() as u16),
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(TRACE_ROWS),
            Constraint::Length(1),
            Constraint::Length(12),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .areas(area);
        f.render_widget(Paragraph::new(logo()), mark.inner(pad));
        f.render_widget(Paragraph::new(tagline_lines()), tagline.inner(pad));
        f.render_widget(Ecg { frame: app.frame }, trace);
        render_menu(f, menu, app.home_sel, true);
        f.render_widget(Paragraph::new(status_note(app)), note.inner(pad));
    } else {
        let [tagline, _, menu, _, note] = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(4),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .areas(area);
        f.render_widget(Paragraph::new(tagline_lines()), tagline.inner(pad));
        render_menu(f, menu, app.home_sel, false);
        f.render_widget(Paragraph::new(status_note(app)), note.inner(pad));
    }
    let switch = match app.env() {
        Env::Test => ("e", "go live"),
        Env::Live => ("e", "back to test"),
    };
    vec![
        ("↑↓", "move"),
        ("enter", "open"),
        ("1–4", "jump"),
        switch,
        ("q", "quit"),
    ]
}

/// The mark in the accent color, the wordmark in the text color.
fn logo() -> Vec<Line<'static>> {
    LOGO.iter()
        .map(|row| {
            let mark: String = row.chars().take(MARK_COLS).collect();
            let word: String = row.chars().skip(MARK_COLS).collect();
            Line::from(vec![
                colored(mark, ACCENT),
                Span::styled(word, Style::new().fg(TEXT)),
            ])
        })
        .collect()
}

fn tagline_lines() -> Vec<Line<'static>> {
    vec![
        Line::from(bold("Naira to stablecoins, and back.")),
        Line::from(dim("Live quotes, one order, tracked until it settles.")),
    ]
}

/// The menu. Tall items get a blank line above and below, so the selection
/// reads as a solid bar.
fn render_menu(f: &mut Frame, area: Rect, selected: usize, tall: bool) {
    let items = ITEMS.iter().enumerate().map(|(i, (title, description))| {
        let on = i == selected;
        let mut line = Line::from(vec![
            Span::styled(
                format!("{}  ", i + 1),
                Style::new().fg(if on { ACCENT } else { FAINT }),
            ),
            Span::styled(
                format!("{title:<20}"),
                Style::new().fg(TEXT).add_modifier(if on {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
            ),
        ]);
        if area.width >= 64 {
            line.push_span(Span::styled(
                *description,
                Style::new().fg(if on { DIM } else { FAINT }),
            ));
        }
        if tall {
            ListItem::new(vec![Line::default(), line, Line::default()])
        } else {
            ListItem::new(line)
        }
    });
    let list = List::new(items)
        .highlight_style(Style::new().bg(RAISED))
        .highlight_symbol(Line::from(colored("▌ ", ACCENT)))
        .highlight_spacing(HighlightSpacing::Always)
        .repeat_highlight_symbol(true);
    let mut state = ListState::default().with_selected(Some(selected));
    f.render_stateful_widget(list, area, &mut state);
}

fn status_note(app: &App) -> Line<'static> {
    let env = app.env().label().to_lowercase();
    if app.key().is_empty() {
        return Line::from(vec![
            colored("!  ", WARN),
            Span::styled(format!("No {env} key yet. "), Style::new().fg(TEXT)),
            dim("Press 4 to add one in Settings."),
        ]);
    }
    match app.env() {
        Env::Test => Line::from(faint("Test mode. Sandbox money only.")),
        Env::Live => Line::from(colored("Live mode. Orders move real money.", ERR)),
    }
}

/// A heart monitor trace in braille. The sweep draws the beat at full
/// strength and lets it fade behind, the way a phosphor screen does.
struct Ecg {
    frame: usize,
}

impl Ecg {
    /// Cells per heartbeat.
    const BEAT: f64 = 34.0;

    /// The height of one heartbeat at phase `u`, from 0 to 1: a soft P wave,
    /// a sharp QRS spike, then a rounder T wave.
    fn height(u: f64) -> f64 {
        const BASE: f64 = 0.32;
        const QRS: [(f64, f64); 5] = [
            (0.24, BASE),
            (0.26, 0.2),
            (0.29, 0.97),
            (0.32, 0.03),
            (0.35, BASE),
        ];
        let bump = |from: f64, len: f64, height: f64| {
            BASE + height * (std::f64::consts::PI * (u - from) / len).sin()
        };
        match u {
            u if (0.08..0.18).contains(&u) => bump(0.08, 0.10, 0.10),
            u if (0.24..0.35).contains(&u) => QRS
                .windows(2)
                .find(|w| u >= w[0].0 && u < w[1].0)
                .map_or(BASE, |w| {
                    let t = (u - w[0].0) / (w[1].0 - w[0].0);
                    w[0].1 + (w[1].1 - w[0].1) * t
                }),
            u if (0.46..0.62).contains(&u) => bump(0.46, 0.16, 0.18),
            _ => BASE,
        }
    }
}

impl Widget for Ecg {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 {
            return;
        }
        let width = area.width as f64;
        let head = (self.frame * 2) as f64 % width;
        let trail = width * 0.7;
        let gap = 4.0;
        let mut strokes: Vec<(f64, f64)> = Vec::new();
        let mut x = 0.0;
        while x + 0.5 <= width {
            let age = (head - x).rem_euclid(width);
            if age < width - gap {
                strokes.push((x, age));
            }
            x += 0.5;
        }
        // Oldest first, so the fresh trace wins braille cells it shares.
        strokes.sort_by(|a, b| b.1.total_cmp(&a.1));
        Canvas::default()
            .marker(Marker::Braille)
            .background_color(BG)
            .x_bounds([0.0, width])
            .y_bounds([0.0, 1.0])
            .paint(|ctx| {
                for &(x, age) in &strokes {
                    let color = if age < 1.0 {
                        TEXT
                    } else if age < trail {
                        mix(ACCENT, BG, (age / trail) as f32)
                    } else {
                        mix(FAINT, BG, 0.55)
                    };
                    ctx.draw(&Stroke {
                        x1: x,
                        y1: Self::height((x / Self::BEAT).fract()),
                        x2: x + 0.5,
                        y2: Self::height(((x + 0.5) / Self::BEAT).fract()),
                        color,
                    });
                }
            })
            .render(area, buf);
    }
}
