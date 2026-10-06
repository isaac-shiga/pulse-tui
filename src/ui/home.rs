use ratatui::{prelude::*, widgets::Paragraph};

use crate::api::Env;
use crate::app::App;

use super::{
    ACCENT, BG, DIM, ERR, FAINT, Hints, RAISED, TEXT, WARN, bold, colored, dim, faint, mix,
};

const WORDMARK: [&str; 3] = ["┏━┓╻ ╻╻  ┏━┓┏━╸", "┣━┛┃ ┃┃  ┗━┓┣╸ ", "╹  ┗━┛┗━╸┗━┛┗━╸"];

const ITEMS: [(&str, &str); 4] = [
    ("Buy stablecoins", "Pay naira, receive USDT or USDC"),
    ("Sell stablecoins", "Send USDT or USDC, receive naira"),
    ("Orders", "Every order and where it stands"),
    ("Settings", "API keys and environment"),
];

/// Cells per heartbeat on the trace.
const BEAT: f64 = 34.0;
/// Rows of the trace.
const TRACE_ROWS: u16 = 4;
/// Shades from empty to solid, for ordered dithering.
const SHADES: [&str; 5] = [" ", "░", "▒", "▓", "█"];
/// A 4×4 Bayer matrix. Each entry is a threshold in sixteenths.
const BAYER: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

pub(super) fn render(f: &mut Frame, area: Rect, app: &App) -> Hints {
    let width = area.width.min(76);
    let area = Rect {
        x: area.x + (area.width - width) / 2,
        width,
        ..area
    };
    if area.height >= 26 && area.width >= 50 {
        full(f, area, app);
    } else {
        compact(f, area, app);
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

fn full(f: &mut Frame, area: Rect, app: &App) {
    let height = 26;
    let area = Rect {
        y: area.y + area.height.saturating_sub(height) / 3,
        height: area.height.min(height),
        ..area
    };
    let [mark, _, tagline, _, trace, _, menu, _, note] = Layout::vertical([
        Constraint::Length(3),
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
    f.render_widget(Paragraph::new(wordmark()), mark.inner(Margin::new(2, 0)));
    f.render_widget(
        Paragraph::new(tagline_lines()),
        tagline.inner(Margin::new(2, 0)),
    );
    ecg(f, trace, app.frame);
    let rows = Layout::vertical([Constraint::Length(3); 4]).split(menu);
    for (i, row) in rows.iter().enumerate() {
        item(f, *row, i, i == app.home_sel, true);
    }
    f.render_widget(
        Paragraph::new(status_note(app)),
        note.inner(Margin::new(2, 0)),
    );
}

fn compact(f: &mut Frame, area: Rect, app: &App) {
    let [tagline, _, menu, _, note] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Length(4),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(area);
    f.render_widget(
        Paragraph::new(tagline_lines()),
        tagline.inner(Margin::new(2, 0)),
    );
    let rows = Layout::vertical([Constraint::Length(1); 4]).split(menu);
    for (i, row) in rows.iter().enumerate() {
        item(f, *row, i, i == app.home_sel, false);
    }
    f.render_widget(
        Paragraph::new(status_note(app)),
        note.inner(Margin::new(2, 0)),
    );
}

fn wordmark() -> Vec<Line<'static>> {
    WORDMARK
        .iter()
        .map(|row| {
            let n = row.chars().count().max(1) as f32;
            Line::from(
                row.chars()
                    .enumerate()
                    .map(|(i, c)| {
                        let color = mix(TEXT, ACCENT, i as f32 / n);
                        Span::styled(c.to_string(), Style::new().fg(color).bold())
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect()
}

fn tagline_lines() -> Vec<Line<'static>> {
    vec![
        Line::from(bold("Naira to stablecoins, and back.")),
        Line::from(dim("Live quotes, one order, tracked until it settles.")),
    ]
}

/// The height of one heartbeat at phase `u`, from 0 to 1: a soft P wave,
/// a sharp QRS spike, then a rounder T wave.
fn beat(u: f64) -> f64 {
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

/// A heart monitor trace. Each cell glows by its distance to the waveform
/// and by how long ago the sweep passed it. Ordered dithering turns the
/// glow into shade blocks, so the trace fades like phosphor.
fn ecg(f: &mut Frame, area: Rect, frame: usize) {
    if area.width < 4 || area.height == 0 {
        return;
    }
    let width = area.width as f64;
    let rows = area.height as f64;
    let head = (frame * 2) as f64 % width;
    let trail = width * 0.7;
    let gap = 3.0;
    let lines = (0..area.height)
        .map(|row| {
            let center = row as f64 + 0.5;
            let spans = (0..area.width)
                .map(|col| {
                    let x = col as f64;
                    let age = (head - x).rem_euclid(width);
                    let strength = if age >= width - gap {
                        0.0
                    } else if age < trail {
                        0.85 - 0.7 * age / trail
                    } else {
                        0.1
                    };
                    // The rows the waveform covers inside this cell.
                    let (lo, hi) = (0..=4)
                        .map(|i| (1.0 - beat(((x + i as f64 / 4.0) / BEAT).fract())) * rows)
                        .fold((f64::MAX, f64::MIN), |(lo, hi), y| (lo.min(y), hi.max(y)));
                    let distance = (lo - center).max(center - hi).max(0.0);
                    let glow = (1.0 - distance / 0.8).clamp(0.0, 1.0) * strength;
                    let level = glow * 4.0;
                    let threshold = (BAYER[row as usize % 4][col as usize % 4] as f64 + 0.5) / 16.0;
                    let shade = (level.floor() + f64::from(u8::from(level.fract() > threshold)))
                        .min(4.0) as usize;
                    let color = if age < 1.0 {
                        TEXT
                    } else {
                        mix(BG, ACCENT, (0.25 + 0.75 * strength) as f32)
                    };
                    Span::styled(SHADES[shade], Style::new().fg(color))
                })
                .collect::<Vec<_>>();
            Line::from(spans)
        })
        .collect::<Vec<_>>();
    f.render_widget(Paragraph::new(lines), area);
}

fn item(f: &mut Frame, area: Rect, index: usize, selected: bool, tall: bool) {
    let (title, description) = ITEMS[index];
    let bg = if selected { RAISED } else { BG };
    f.render_widget(
        ratatui::widgets::Block::new().style(Style::new().bg(bg)),
        area,
    );
    let bar = if selected {
        colored("▌", ACCENT)
    } else {
        Span::raw(" ")
    };
    let title_style = if selected {
        Style::new().fg(TEXT).bold()
    } else {
        Style::new().fg(TEXT)
    };
    let number = if selected {
        colored(format!(" {}  ", index + 1), ACCENT)
    } else {
        faint(format!(" {}  ", index + 1))
    };
    let mut line = Line::from(vec![
        bar,
        number,
        Span::styled(format!("{title:<20}"), title_style),
    ]);
    if area.width >= 64 {
        line.push_span(Span::styled(
            description,
            Style::new().fg(if selected { DIM } else { FAINT }),
        ));
    }
    let blank = Line::from(if selected {
        colored("▌", ACCENT)
    } else {
        Span::raw(" ")
    });
    let lines = if tall {
        vec![blank.clone(), line, blank]
    } else {
        vec![line]
    };
    f.render_widget(Paragraph::new(lines), area);
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
