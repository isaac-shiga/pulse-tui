use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Tabs, Wrap},
};

use crate::api::{Order, Status};
use crate::app::{App, Copyable, Mark, Tracker};

use super::components::{Ledger, Row as LedgerRow, heading, panel};
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
    let (funding, progress) = if area.width >= 80 {
        let area = Rect {
            height: area.height.min(20),
            ..area
        };
        let [left, right] = Layout::horizontal([Constraint::Fill(55), Constraint::Fill(45)])
            .spacing(2)
            .areas(area);
        (left, right)
    } else {
        let [top, bottom] = Layout::vertical([Constraint::Length(14), Constraint::Length(20)])
            .spacing(1)
            .areas(area);
        (top, bottom)
    };
    f.render_widget(
        FundingPanel {
            tracker: t,
            spinner: spinner(app),
        },
        funding,
    );
    f.render_widget(
        ProgressPanel {
            tracker: t,
            spinner: spinner(app),
        },
        progress,
    );
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

/// Where and how much to pay while the order waits, and a record after.
struct FundingPanel<'a> {
    tracker: &'a Tracker,
    spinner: Span<'static>,
}

impl Widget for FundingPanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let t = self.tracker;
        let o = &t.order;
        let awaiting = o.status == Status::AwaitingPayment;
        let title = match (awaiting, o.is_onramp()) {
            (true, true) => "Pay from your bank",
            (true, false) => "Send stablecoins",
            (false, _) => "Payment",
        };
        let block = panel(title, None);
        let inner = block.inner(area);
        block.render(area, buf);
        let fa = o.funding_account.clone().unwrap_or_default();
        let picked = |c| t.pick == c;
        let loading = |waiting: &'static str| {
            LedgerRow::text(vec![
                self.spinner.clone(),
                dim(if t.fresh {
                    waiting
                } else {
                    " Loading the order…"
                }),
            ])
        };
        let amount_style = if awaiting {
            Style::new().fg(ACCENT).bold()
        } else {
            Style::new().fg(TEXT).bold()
        };
        let amount = money(
            &o.source.currency,
            &t.value(Copyable::Amount).unwrap_or_default(),
        );
        let network = fa
            .network
            .clone()
            .or(o.source.network.clone())
            .map(|n| network_name(&n));
        let on = match (&network, o.is_onramp()) {
            (Some(n), false) => format!(" on {n}"),
            _ => String::new(),
        };
        let mut rows = vec![
            LedgerRow::Pair(
                copy_label(
                    if awaiting { "Send exactly" } else { "Amount" },
                    picked(Copyable::Amount),
                ),
                Line::from(vec![
                    copy_value(amount, amount_style, picked(Copyable::Amount)),
                    dim(on),
                ]),
            ),
            LedgerRow::Gap,
        ];
        if o.is_onramp() {
            match fa.account_number {
                // Pulse issues the account only while the order waits for money.
                None if awaiting => rows.push(loading(" Opening an account for this order…")),
                None => {}
                Some(number) => {
                    let bank = match (fa.bank_name, fa.bank_code) {
                        (Some(n), Some(c)) => format!("{n} · {c}"),
                        (Some(n), None) => n,
                        _ => String::new(),
                    };
                    rows.push(LedgerRow::pair("Bank", bank));
                    rows.push(LedgerRow::Pair(
                        copy_label("Account", picked(Copyable::Funding)),
                        Line::from(copy_value(
                            number,
                            Style::new().fg(TEXT).bold(),
                            picked(Copyable::Funding),
                        )),
                    ));
                    rows.push(LedgerRow::pair("Name", fa.account_name.unwrap_or_default()));
                    if awaiting {
                        rows.push(LedgerRow::Gap);
                        rows.push(LedgerRow::text(vec![
                            colored("!  ", WARN),
                            dim("Send this exact amount. Your bank rejects any other."),
                        ]));
                    }
                }
            }
        } else {
            match fa.deposit_address {
                None if awaiting => rows.push(loading(" Waiting for the deposit address…")),
                None => {}
                Some(address) => {
                    // An address is too long to share a row with its label.
                    rows.push(LedgerRow::text(copy_label(
                        if awaiting {
                            "To this address"
                        } else {
                            "Deposit address"
                        },
                        picked(Copyable::Funding),
                    )));
                    rows.push(LedgerRow::text(copy_value(
                        address,
                        Style::new().fg(TEXT).bold(),
                        picked(Copyable::Funding),
                    )));
                }
            }
            if awaiting {
                let network = network.unwrap_or_default();
                rows.push(LedgerRow::Gap);
                rows.push(LedgerRow::text(vec![
                    colored("!  ", WARN),
                    colored(
                        format!("Use {network} only. Coins sent on another network are lost."),
                        WARN,
                    ),
                ]));
            }
        }
        if awaiting && let Some(s) = o.seconds_until_expiry() {
            let color = if s <= 300 { WARN } else { TEXT };
            rows.push(LedgerRow::Gap);
            rows.push(LedgerRow::pair(
                "Closes in",
                Span::styled(clock(s), Style::new().fg(color).bold()),
            ));
        }
        Ledger(rows).render(inner, buf);
    }
}

/// The status timeline, what the order delivers, and its IDs.
struct ProgressPanel<'a> {
    tracker: &'a Tracker,
    spinner: Span<'static>,
}

impl Widget for ProgressPanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let t = self.tracker;
        let o = &t.order;
        // The list sends a summary. The spinner shows until the full order lands.
        let block = panel("Progress", (!t.fresh).then(|| self.spinner.clone()));
        let inner = block.inner(area);
        block.render(area, buf);
        let mut rows = Vec::new();
        for (i, (s, mark)) in t.timeline().iter().enumerate() {
            if i > 0 {
                let lit = matches!(mark, Mark::Done | Mark::Current | Mark::Bad);
                rows.push(LedgerRow::text(colored(
                    "│",
                    if lit { ACCENT } else { LINE },
                )));
            }
            rows.push(LedgerRow::text(match mark {
                Mark::Done => Line::from(vec![colored("✓  ", OK), dim(s.label())]),
                Mark::Current => Line::from(vec![
                    self.spinner.clone(),
                    Span::raw(" "),
                    Span::styled(s.label(), Style::new().fg(TEXT).bold()),
                ]),
                Mark::Todo => Line::from(faint(format!("○  {}", s.label()))),
                Mark::Bad => Line::from(vec![
                    colored("✕  ", ERR),
                    Span::styled(s.label(), Style::new().fg(ERR).bold()),
                ]),
            }));
        }
        rows.push(LedgerRow::Gap);
        let dest = &o.destination;
        let deliver = match &dest.network {
            Some(n) => format!(
                "{} on {}",
                money(&dest.currency, &dest.amount),
                network_name(n)
            ),
            None => money(&dest.currency, &dest.amount),
        };
        rows.push(LedgerRow::pair(
            if o.is_onramp() { "You get" } else { "Payout" },
            bold(deliver),
        ));
        if let Some(rate) = &o.rate {
            rows.push(LedgerRow::pair("Rate", format!("₦{}", group(rate))));
        }
        if let Some(name) = o.party.as_ref().and_then(|p| p.name.clone()) {
            rows.push(LedgerRow::pair("For", name));
        }
        rows.push(LedgerRow::Divider);
        // IDs can be long, so each sits on its own line under its label.
        for (c, label, value) in [
            (Copyable::Reference, "Reference", o.reference.clone()),
            (Copyable::OrderId, "Order ID", o.id.clone()),
        ] {
            rows.push(LedgerRow::text(copy_label(label, t.pick == c)));
            rows.push(LedgerRow::text(copy_value(
                value,
                Style::new().fg(TEXT),
                t.pick == c,
            )));
        }
        rows.push(LedgerRow::Gap);
        rows.push(LedgerRow::text(match (o.status, &t.error) {
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
        }));
        Ledger(rows).render(inner, buf);
    }
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
    let tabs_block = Block::new()
        .borders(Borders::BOTTOM)
        .border_style(Style::new().fg(LINE));
    let tab_row = tabs_block.inner(tabs);
    f.render_widget(
        Tabs::new(["All", "Buys", "Sells"])
            .select(active)
            .style(Style::new().fg(DIM))
            .highlight_style(Style::new().fg(ACCENT).bold())
            .divider(" ")
            .padding(" ", " ")
            .block(tabs_block),
        tabs,
    );
    if view.loading {
        f.render_widget(Line::from(spinner(app)).right_aligned(), tab_row);
    }

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
            (
                colored("○", ACCENT),
                title,
                "Orders show up here once you place one.",
            )
        };
        let [_, empty] =
            Layout::vertical([Constraint::Length(table.height / 4), Constraint::Fill(1)])
                .areas(table);
        f.render_widget(
            Paragraph::new(vec![
                Line::from(icon),
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
