use ratatui::{
    prelude::*,
    widgets::{Paragraph, Wrap},
};

use crate::api::Env;
use crate::app::App;
use crate::config::Config;

use super::components::{field, heading, input, panel, segmented, under_field};
use super::{ACCENT, Hints, TEXT, bold, dim, faint};

pub(super) fn render(f: &mut Frame, area: Rect, app: &App) -> Hints {
    let s = &app.settings;
    let env = if s.env == Env::Test { 0 } else { 1 };
    let form = vec![
        field(
            "Environment",
            s.focus == 0,
            segmented(&["Test", "Live"], env, s.focus == 0),
            None,
        ),
        under_field(vec![faint(s.env.base_url())]),
        Line::default(),
        field(
            "Test key",
            s.focus == 1,
            input(&s.test_key, s.focus == 1, "sk_test_…", 36),
            None,
        ),
        Line::default(),
        field(
            "Live key",
            s.focus == 2,
            input(&s.live_key, s.focus == 2, "sk_live_…", 36),
            None,
        ),
    ];
    let [head, _, body] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .areas(area);
    f.render_widget(
        Paragraph::new(heading(
            "Settings",
            "Pick where orders run, and add the key for it.",
        )),
        head,
    );
    let (fields, info) = if body.width >= 84 {
        let [fields, info] = Layout::horizontal([Constraint::Fill(64), Constraint::Fill(36)])
            .spacing(3)
            .areas(body);
        (fields, info)
    } else {
        let [fields, info] = Layout::vertical([Constraint::Length(10), Constraint::Fill(1)])
            .spacing(1)
            .areas(body);
        (fields, info)
    };
    let fields = Rect {
        height: fields.height.min(10),
        ..fields
    };
    let block = panel("Environment and keys", None);
    let inner = block.inner(fields);
    f.render_widget(block, fields);
    f.render_widget(Paragraph::new(form), inner);
    f.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                "Where keys live",
                Style::new().fg(ACCENT).bold(),
            )),
            Line::default(),
            Line::from(Span::styled(
                "On this machine, in a file only your user can read.",
                Style::new().fg(TEXT),
            )),
            Line::from(faint(Config::path().display().to_string())),
            Line::default(),
            Line::from(vec![
                dim("Leave a field empty to use "),
                bold("PULSE_TEST_KEY"),
                dim(" or "),
                bold("PULSE_LIVE_KEY"),
                dim(" from your shell."),
            ]),
        ])
        .wrap(Wrap { trim: true }),
        info.inner(Margin::new(1, 1)),
    );
    vec![
        ("↑↓", "move"),
        ("←→", "environment"),
        ("del", "clear"),
        ("enter", "save"),
        ("esc", "discard"),
    ]
}
