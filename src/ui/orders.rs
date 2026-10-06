use ratatui::{
    prelude::*,
    widgets::{Cell, Paragraph, Row, Table, TableState, Wrap},
};

use crate::api::{Order, Status};
use crate::app::{App, Copyable, Mark, Tracker};

use super::components::{divider, heading, pair, pair_span, panel};
use super::format::{clock, group, money, network_name, short_time};
use super::{
    ACCENT, DIM, ERR, FAINT, Hints, LINE, OK, RAISED, TEXT, WARN, bold, colored, dim, faint,
    spinner,
};

/// The title and subtitle that say what the order needs next.
pub(super) fn order_heading(o: &Order) -> (String, String) {
    let (title, subtitle) = match (o.status, o.is_onramp()) {
        (Status::AwaitingPayment, true) => (
            "Send the naira",
            "Pay from your bank to the account below. The order moves on by itself.",
        ),
        (Status::AwaitingPayment, false) => (
            "Send the coins",
            "Send to the address below. The order moves on by itself.",
        ),
        (Status::Processing, _) => (
            "Payment received",
            "Pulse is settling the order. You can leave this screen.",
        ),
        (Status::Completed, _) => ("Settled", "The order is complete."),
        (Status::Expired, _) => (
            "This order expired",
            "No payment arrived in time. Start a new order for a fresh price.",
        ),
        (Status::Failed, _) => (
            "This order failed",
            "Contact support@shiga.io and include the order ID.",
        ),
        (Status::Unknown, _) => (
            "Checking the order",
            "Pulse sent a status this app does not know.",
        ),
    };
    (title.to_string(), subtitle.to_string())
}

pub(super) fn order_view(f: &mut Frame, area: Rect, t: &Tracker, app: &App) {
    if area.width >= 80 {
        let area = Rect {
            height: area.height.min(20),
            ..area
        };
        let [left, right] = Layout::horizontal([Constraint::Fill(55), Constraint::Fill(45)])
            .spacing(2)
            .areas(area);
        funding(f, left, t, app);
        status(f, right, t, app);
    } else {
        let [funding_area, status_area] =
            Layout::vertical([Constraint::Length(14), Constraint::Length(20)])
                .spacing(1)
                .areas(area);
        funding(f, funding_area, t, app);
        status(f, status_area, t, app);
    }
}

/// A label that lights up when its value is the one `c` copies.
fn copy_label(label: &'static str, picked: bool) -> Span<'static> {
    if picked {
        Span::styled(format!("▸ {label}"), Style::new().fg(ACCENT).bold())
    } else {
        dim(label)
    }
}

/// A copyable value. The picked one sits on a lit background.
fn copy_value(text: String, style: Style, picked: bool) -> Span<'static> {
    if picked {
        Span::styled(text, style.bg(RAISED))
    } else {
        Span::styled(text, style)
    }
}

fn funding(f: &mut Frame, area: Rect, t: &Tracker, app: &App) {
    let o = &t.order;
    let awaiting = o.status == Status::AwaitingPayment;
    let title = match (awaiting, o.is_onramp()) {
        (true, true) => "Pay from your bank",
        (true, false) => "Send stablecoins",
        (false, _) => "Payment",
    };
    let block = panel(title);
    let inner = block.inner(area);
    f.render_widget(block, area);
    let w = inner.width;
    let fa = o.funding_account.clone().unwrap_or_default();
    let picked = |c| t.pick == c;
    let amount_style = if awaiting {
        Style::new().fg(ACCENT).bold()
    } else {
        Style::new().fg(TEXT).bold()
    };
    let mut lines = Vec::new();
    let amount = t.value(Copyable::Amount).unwrap_or_default();
    let amount = money(&o.source.currency, &amount);
    let network = fa
        .network
        .clone()
        .or(o.source.network.clone())
        .map(|n| network_name(&n));
    let on = match (&network, o.is_onramp()) {
        (Some(n), false) => format!(" on {n}"),
        _ => String::new(),
    };
    lines.push(pair_span(
        copy_label(
            if awaiting { "Send exactly" } else { "Amount" },
            picked(Copyable::Amount),
        ),
        vec![
            copy_value(amount, amount_style, picked(Copyable::Amount)),
            dim(on),
        ],
        w,
    ));
    lines.push(Line::default());
    if o.is_onramp() {
        match fa.account_number {
            // Pulse issues the account only while the order waits for money.
            None if awaiting => lines.push(Line::from(vec![
                colored(format!("{}  ", spinner(app)), ACCENT),
                dim("Opening an account for this order…"),
            ])),
            None => {}
            Some(number) => {
                let bank = match (fa.bank_name, fa.bank_code) {
                    (Some(n), Some(c)) => format!("{n} · {c}"),
                    (Some(n), None) => n,
                    _ => String::new(),
                };
                lines.push(pair("Bank", vec![Span::raw(bank)], w));
                lines.push(pair_span(
                    copy_label("Account", picked(Copyable::Funding)),
                    vec![copy_value(
                        number,
                        Style::new().fg(TEXT).bold(),
                        picked(Copyable::Funding),
                    )],
                    w,
                ));
                lines.push(pair(
                    "Name",
                    vec![Span::raw(fa.account_name.unwrap_or_default())],
                    w,
                ));
                if awaiting {
                    lines.push(Line::default());
                    lines.push(Line::from(vec![
                        colored("!  ", WARN),
                        dim("Send this exact amount. Your bank rejects any other."),
                    ]));
                }
            }
        }
    } else {
        match fa.deposit_address {
            None if awaiting => lines.push(Line::from(vec![
                colored(format!("{}  ", spinner(app)), ACCENT),
                dim("Waiting for the deposit address…"),
            ])),
            None => {}
            Some(address) => {
                lines.push(Line::from(copy_label(
                    if awaiting {
                        "To this address"
                    } else {
                        "Deposit address"
                    },
                    picked(Copyable::Funding),
                )));
                lines.push(Line::from(copy_value(
                    address,
                    Style::new().fg(TEXT).bold(),
                    picked(Copyable::Funding),
                )));
            }
        }
        if awaiting {
            let network = network.unwrap_or_default();
            lines.push(Line::default());
            lines.push(Line::from(vec![
                colored("!  ", WARN),
                colored(
                    format!("Use {network} only. Coins sent on another network are lost."),
                    WARN,
                ),
            ]));
        }
    }
    if awaiting && let Some(s) = o.seconds_until_expiry() {
        lines.push(Line::default());
        let color = if s <= 300 { WARN } else { TEXT };
        lines.push(Line::from(vec![
            dim("Closes in "),
            Span::styled(clock(s), Style::new().fg(color).bold()),
        ]));
    }
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn status(f: &mut Frame, area: Rect, t: &Tracker, app: &App) {
    let o = &t.order;
    let block = panel("Progress");
    let inner = block.inner(area);
    f.render_widget(block, area);
    let w = inner.width;
    let mut lines = Vec::new();
    let timeline = t.timeline();
    for (i, (s, mark)) in timeline.iter().enumerate() {
        if i > 0 {
            let lit = matches!(mark, Mark::Done | Mark::Current | Mark::Bad);
            lines.push(Line::from(colored("│", if lit { ACCENT } else { LINE })));
        }
        lines.push(match mark {
            Mark::Done => Line::from(vec![colored("✓  ", OK), dim(s.label())]),
            Mark::Current => Line::from(vec![
                colored(format!("{}  ", spinner(app)), ACCENT),
                Span::styled(s.label(), Style::new().fg(TEXT).bold()),
            ]),
            Mark::Todo => Line::from(faint(format!("○  {}", s.label()))),
            Mark::Bad => Line::from(vec![
                colored("✕  ", ERR),
                Span::styled(s.label(), Style::new().fg(ERR).bold()),
            ]),
        });
    }
    lines.push(Line::default());
    let dest = &o.destination;
    let deliver = match &dest.network {
        Some(n) => format!(
            "{} on {}",
            money(&dest.currency, &dest.amount),
            network_name(n)
        ),
        None => money(&dest.currency, &dest.amount),
    };
    lines.push(pair(
        if o.is_onramp() { "You get" } else { "Payout" },
        vec![bold(deliver)],
        w,
    ));
    if let Some(rate) = &o.rate {
        lines.push(pair(
            "Rate",
            vec![Span::raw(format!("₦{}", group(rate)))],
            w,
        ));
    }
    if let Some(name) = o.party.as_ref().and_then(|p| p.name.clone()) {
        lines.push(pair("For", vec![Span::raw(name)], w));
    }
    lines.push(divider(w));
    // IDs can be long, so each sits on its own line under its label.
    for (c, label, value) in [
        (Copyable::Reference, "Reference", o.reference.clone()),
        (Copyable::OrderId, "Order ID", o.id.clone()),
    ] {
        lines.push(Line::from(copy_label(label, t.pick == c)));
        lines.push(Line::from(copy_value(
            value,
            Style::new().fg(TEXT),
            t.pick == c,
        )));
    }
    lines.push(Line::default());
    lines.push(match (o.status, &t.error) {
        (_, Some(e)) if !o.status.is_terminal() => {
            Line::from(vec![colored("✕  ", ERR), colored(e.friendly(), ERR)])
        }
        (Status::Completed, _) => Line::from(colored("✓  Settled.", OK)),
        (Status::Expired, _) => Line::from(colored("Not funded in time.", WARN)),
        (Status::Failed, _) => Line::from(colored("Pulse could not settle this order.", ERR)),
        _ => Line::from(vec![
            colored("● ", ACCENT),
            faint("Live. Checks every 5 seconds."),
        ]),
    });
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn status_color(s: Status) -> Color {
    match s {
        Status::AwaitingPayment => WARN,
        Status::Processing => ACCENT,
        Status::Completed => OK,
        Status::Failed => ERR,
        Status::Expired | Status::Unknown => DIM,
    }
}

pub(super) fn render(f: &mut Frame, area: Rect, app: &App) -> Hints {
    let view = &app.orders;
    if let Some(t) = &view.detail {
        let [head, _, body] = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .areas(area);
        let (title, subtitle) = order_heading(&t.order);
        f.render_widget(
            Paragraph::new(heading(title, subtitle)).wrap(Wrap { trim: true }),
            head,
        );
        order_view(f, body, t, app);
        return vec![("↑↓", "choose"), ("c", "copy"), ("esc", "back")];
    }
    let [head, _, tabs, _, table] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .areas(area);
    f.render_widget(
        Paragraph::new(heading(
            "Orders",
            "Newest first. Open one to follow it live.",
        )),
        head,
    );
    let active = match view.filter {
        None => 0,
        Some("onramp") => 1,
        Some(_) => 2,
    };
    let mut labels = Vec::new();
    let mut rule = Vec::new();
    for (i, name) in ["All", "Buys", "Sells"].iter().enumerate() {
        let on = i == active;
        let style = if on {
            Style::new().fg(TEXT).bold()
        } else {
            Style::new().fg(DIM)
        };
        labels.push(Span::styled(format!(" {name} "), style));
        labels.push(Span::raw("  "));
        rule.push(colored(
            "━".repeat(name.len() + 2),
            if on { ACCENT } else { LINE },
        ));
        rule.push(colored("━━", LINE));
    }
    if view.loading {
        labels.push(colored(spinner(app), ACCENT));
    }
    let used: usize = rule.iter().map(Span::width).sum();
    rule.push(colored(
        "━".repeat((tabs.width as usize).saturating_sub(used)),
        LINE,
    ));
    f.render_widget(
        Paragraph::new(vec![Line::from(labels), Line::from(rule)]),
        tabs,
    );

    if let Some(e) = &view.error {
        f.render_widget(
            Paragraph::new(vec![
                Line::from(vec![colored("✕  ", ERR), bold(e.friendly())]),
                Line::from(faint("   Press r to try again.")),
            ])
            .wrap(Wrap { trim: false }),
            table,
        );
    } else if view.items.is_empty() {
        let (icon, title, body) = if view.loading {
            (spinner(app), "Loading your orders…", "This takes a moment.")
        } else {
            let title = match active {
                0 => "No orders yet",
                1 => "No buys yet",
                _ => "No sells yet",
            };
            ("○", title, "Orders show up here once you place one.")
        };
        let [_, empty] =
            Layout::vertical([Constraint::Length(table.height / 4), Constraint::Fill(1)])
                .areas(table);
        f.render_widget(
            Paragraph::new(vec![
                Line::from(colored(icon, ACCENT)),
                Line::default(),
                Line::from(bold(title)),
                Line::from(dim(body)),
            ])
            .alignment(Alignment::Center),
            empty,
        );
    } else {
        let right = |s: String, style: Style| {
            Cell::from(Line::from(Span::styled(s, style)).right_aligned())
        };
        let rows = view.items.iter().map(|o| {
            let kind = if o.is_onramp() {
                colored("Buy", TEXT)
            } else {
                colored("Sell", TEXT)
            };
            Row::new(vec![
                Cell::from(dim(o
                    .created_at
                    .as_deref()
                    .map(short_time)
                    .unwrap_or_default())),
                Cell::from(kind),
                Cell::from(Line::from(vec![
                    colored("● ", status_color(o.status)),
                    Span::raw(o.status.label()),
                ])),
                right(
                    money(&o.source.currency, &o.source.amount),
                    Style::new().fg(TEXT),
                ),
                right(
                    money(&o.destination.currency, &o.destination.amount),
                    Style::new().fg(TEXT).bold(),
                ),
                Cell::from(faint(o.reference.clone())),
            ])
        });
        let widths = [
            Constraint::Length(12),
            Constraint::Length(4),
            Constraint::Length(18),
            Constraint::Length(15),
            Constraint::Length(15),
            Constraint::Fill(1),
        ];
        let header = Row::new(vec![
            Cell::from("Created"),
            Cell::from("Type"),
            Cell::from("Status"),
            Cell::from(Line::from("Send").right_aligned()),
            Cell::from(Line::from("Receive").right_aligned()),
            Cell::from("Reference"),
        ])
        .style(Style::new().fg(FAINT))
        .bottom_margin(1);
        let widget = Table::new(rows, widths)
            .header(header)
            .column_spacing(2)
            .row_highlight_style(Style::new().bg(RAISED))
            .highlight_symbol(Span::styled("▌ ", Style::new().fg(ACCENT)));
        let mut state = TableState::default().with_selected(Some(view.sel));
        f.render_stateful_widget(widget, table, &mut state);
    }
    let mut hints = vec![
        ("↑↓", "select"),
        ("enter", "open"),
        ("t", "filter"),
        ("r", "reload"),
    ];
    if view.cursor.is_some() {
        hints.push(("m", "load more"));
    }
    hints.push(("esc", "back"));
    hints
}
