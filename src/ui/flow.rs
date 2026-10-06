use ratatui::{
    prelude::*,
    widgets::{Paragraph, Wrap},
};

use crate::api::Env;
use crate::app::sandbox::{self, OUTCOMES};
use crate::app::{ASSETS, App, Dir, Field, Flow, Input, Limit, Step, networks_for};

use super::components::{
    Countdown, INPUT_WIDTH, Ledger, Row, VALUE_COLUMN, field, heading, input, keycap, panel,
    segmented, selector, text_box, under_field,
};
use super::format::{group, mask, money, network_name};
use super::orders::{order_heading, order_view};
use super::{
    ACCENT, BG, DIM, ERR, FAINT, Hints, LINE, OK, TEXT, WARN, bold, colored, dim, faint, spinner,
};

const FORM_WIDTH: u16 = 80;

pub(super) fn render(f: &mut Frame, area: Rect, app: &App) -> Hints {
    let (title, subtitle) = step_heading(app);
    let head_text = Paragraph::new(heading(title, subtitle)).wrap(Wrap { trim: true });
    // A subtitle can wrap on a narrow terminal, so the heading takes the rows it needs.
    let head_rows = head_text.line_count(area.width) as u16;
    let [steps, _, head, _, content] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Length(head_rows),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .areas(area);
    f.render_widget(Stepper(&app.flow), steps);
    f.render_widget(head_text, head);
    let form = form_hints(app);
    let narrow = Rect {
        width: content.width.min(FORM_WIDTH),
        ..content
    };
    match app.flow.step() {
        Step::Amount => {
            amount_step(f, content, app);
            form
        }
        Step::Party => {
            party_step(f, narrow, app);
            form
        }
        Step::Wallet => {
            wallet_step(f, narrow, app);
            form
        }
        Step::Bank => {
            bank_step(f, narrow, app);
            form
        }
        Step::Review => {
            review_step(f, narrow, app);
            let place = if app.env() == Env::Live {
                "place live order"
            } else {
                "place order"
            };
            vec![("enter", place), ("esc", "back")]
        }
        Step::Track => {
            if let Some(t) = &app.flow.tracker {
                order_view(f, content, t, app);
            }
            let save = if app.flow.dir == Dir::On {
                "save payer"
            } else {
                "save recipient"
            };
            let mut hints = vec![("↑↓", "choose"), ("c", "copy")];
            if !app.profile_saved() {
                hints.push(("s", save));
            }
            hints.extend([("n", "new order"), ("enter", "done")]);
            hints
        }
    }
}

fn step_heading(app: &App) -> (String, String) {
    let fl = &app.flow;
    let test = app.env() == Env::Test;
    let (title, subtitle) = match (fl.step(), fl.dir) {
        (Step::Amount, _) => ("How much?", "Rates update live as you type."),
        (Step::Party, Dir::On) => (
            "Who is paying?",
            "Use the details of the person who sends the naira.",
        ),
        (Step::Party, Dir::Off) => (
            "Who is receiving it?",
            "Use the details of the account holder. We filled in the name the bank gave.",
        ),
        (Step::Wallet, _) if test => (
            "Where should the coins go?",
            "Test mode uses reserved addresses. Each one plays out a different ending.",
        ),
        (Step::Wallet, _) => (
            "Where should the coins go?",
            "Paste an address you control. Coins sent to a wrong address cannot be recovered.",
        ),
        (Step::Bank, _) if test => (
            "Where should the naira go?",
            "Pick a bank, then an ending. Test mode fills in a reserved account.",
        ),
        (Step::Bank, _) => (
            "Where should the naira go?",
            "Pick the bank and type the account number. The bank confirms the name.",
        ),
        (Step::Review, _) => (
            "One last look",
            "Check every line. The order uses this exact price.",
        ),
        (Step::Track, _) => {
            return fl
                .tracker
                .as_ref()
                .map(|t| order_heading(&t.order))
                .unwrap_or_default();
        }
    };
    (title.to_string(), subtitle.to_string())
}

/// Step labels over a progress rule that fills as the order moves on.
struct Stepper<'a>(&'a Flow);

impl Widget for Stepper<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let fl = self.0;
        let mut labels = Vec::new();
        let mut rule = Vec::new();
        let gap = "   ";
        for (i, step) in fl.dir.steps().iter().enumerate() {
            let lit = i <= fl.step_idx;
            let color = if lit { ACCENT } else { LINE };
            if i > 0 {
                labels.push(Span::raw(gap));
                rule.push(colored("━".repeat(gap.len()), color));
            }
            let label = step.label(fl.dir);
            let (mark, text) = match i.cmp(&fl.step_idx) {
                std::cmp::Ordering::Less => (colored("✓ ", OK), dim(label)),
                std::cmp::Ordering::Equal => (colored("● ", ACCENT), bold(label)),
                std::cmp::Ordering::Greater => (faint("○ "), faint(label)),
            };
            rule.push(colored("━".repeat(label.chars().count() + 2), color));
            labels.extend([mark, text]);
        }
        let used: usize = rule.iter().map(Span::width).sum();
        let tail = if fl.step() == Step::Track {
            ACCENT
        } else {
            LINE
        };
        rule.push(colored(
            "━".repeat((area.width as usize).saturating_sub(used)),
            tail,
        ));
        Paragraph::new(vec![Line::from(labels), Line::from(rule)]).render(area, buf);
    }
}

fn form_hints(app: &App) -> Hints {
    let mut hints = vec![("↑↓", "move")];
    if matches!(
        app.flow.focus,
        Field::Asset | Field::Network | Field::Saved | Field::Outcome
    ) {
        hints.push(("←→", "change"));
    }
    if app.flow.focus == Field::Amount {
        hints.push(("←→", "swap side"));
    }
    if matches!(
        app.flow.focus,
        Field::Amount
            | Field::Name
            | Field::Email
            | Field::Nin
            | Field::Bvn
            | Field::Address
            | Field::Bank
            | Field::Account
    ) {
        hints.push(("del", "clear"));
    }
    hints.extend([("enter", "continue"), ("esc", "back")]);
    hints
}

fn blank() -> Line<'static> {
    Line::default()
}

/// Maps a cursor in the raw amount to the same place in its grouped form.
fn grouped_cursor(grouped: &str, cursor: usize) -> usize {
    let mut digits = 0;
    for (i, c) in grouped.chars().enumerate() {
        if digits == cursor {
            return i;
        }
        if c != ',' {
            digits += 1;
        }
    }
    grouped.chars().count()
}

fn amount_step(f: &mut Frame, area: Rect, app: &App) {
    let fl = &app.flow;
    let focus = |x| fl.focus == x;
    let (label, unit) = fl.typed_side();
    let shown = group(fl.amount.value());
    let cursor = grouped_cursor(&shown, fl.amount.cursor());
    let mut amount = Vec::new();
    if unit == "NGN" {
        amount.push(bold("₦ "));
    }
    amount.extend(text_box(shown, cursor, focus(Field::Amount), "0", 18));
    if unit != "NGN" {
        amount.push(bold(format!(" {unit}")));
    }
    let networks = networks_for(fl.asset);
    let network_pos = networks.iter().position(|n| *n == fl.network).unwrap_or(0);
    let asset_pos = ASSETS.iter().position(|a| *a == fl.asset).unwrap_or(0);
    let other = if fl.fix_ngn { fl.asset } else { "naira" };
    let mut lines = vec![
        field(
            "Coin",
            focus(Field::Asset),
            segmented(&ASSETS, asset_pos, focus(Field::Asset)),
            None,
        ),
        blank(),
        field(
            "Network",
            focus(Field::Network),
            selector(&network_name(fl.network), focus(Field::Network)),
            focus(Field::Network)
                .then(|| faint(format!("{} of {}", network_pos + 1, networks.len()))),
        ),
        blank(),
        field(label, focus(Field::Amount), amount, None),
    ];
    // The limit shows once typing pauses, so it does not flash on every key.
    if let Some(limit) = fl.amount_limit().filter(|_| fl.quote_due.is_none()) {
        let text = match limit {
            Limit::Min(currency, amount) => format!("Minimum {}", money(currency, amount)),
            Limit::Max(currency, amount) => format!("Maximum {}", money(currency, amount)),
        };
        lines.push(under_field(vec![colored(text, WARN)]));
    }
    if focus(Field::Amount) {
        lines.push(under_field(vec![faint(format!("← → type in {other}"))]));
    }
    let (form, card) = if area.width >= 80 {
        let area = Rect {
            height: area.height.min(11),
            ..area
        };
        let [form, card] = Layout::horizontal([Constraint::Fill(46), Constraint::Fill(54)])
            .spacing(2)
            .areas(area);
        (form, card)
    } else {
        let area = Rect {
            height: area.height.min(21),
            ..area
        };
        let [form, card] = Layout::vertical([Constraint::Length(10), Constraint::Length(10)])
            .spacing(1)
            .areas(area);
        (form, card)
    };
    let block = panel("Amount", None);
    let inner = block.inner(form);
    f.render_widget(block, form);
    f.render_widget(Paragraph::new(lines), inner);
    f.render_widget(
        QuoteCard {
            flow: fl,
            spinner: spinner(app),
        },
        card,
    );
}

/// The live price for the amount, laid out like a receipt.
struct QuoteCard<'a> {
    flow: &'a Flow,
    spinner: Span<'static>,
}

impl Widget for QuoteCard<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let fl = self.flow;
        let limit = fl.amount_limit();
        // An amount outside the limits gets no request, so nothing is pending for it.
        let pending = (fl.quote_loading || fl.quote_due.is_some()) && fl.priceable();
        let quote = fl.quote.as_ref().filter(|_| fl.priceable());
        let block = panel("Quote", pending.then(|| self.spinner.clone()));
        let inner = block.inner(area);
        block.render(area, buf);
        let rows = if let Some(e) = &fl.quote_err {
            vec![
                Row::text(vec![
                    colored("✕  ", ERR),
                    Span::styled(
                        e.friendly_or("Pulse could not price this amount. Try again."),
                        Style::new().fg(TEXT),
                    ),
                ]),
                Row::Gap,
                Row::text(faint(format!("Code: {}", e.code))),
            ]
        } else if let Some(q) = quote {
            let (src, dst) = fl.currencies();
            vec![
                Row::pair("You pay", bold(money(src, &q.source_amount))),
                Row::pair(
                    "You get",
                    Span::styled(
                        money(dst, &q.destination_amount),
                        Style::new().fg(ACCENT).bold(),
                    ),
                ),
                Row::Divider,
                Row::pair("Rate", format!("₦{} per {}", group(&q.rate), fl.asset)),
                Row::pair("Network", network_name(fl.network)),
                Row::Gap,
                if pending {
                    Row::text(faint("Updating the price…"))
                } else {
                    Row::text(vec![
                        colored("● ", ACCENT),
                        faint("Live rate. Final price on Review."),
                    ])
                },
            ]
        } else if pending {
            vec![Row::text(vec![
                self.spinner,
                dim(" Fetching a live price…"),
            ])]
        } else {
            let limits = match fl.dir {
                Dir::On => "Buy from ₦15,000 up to ₦100,000,000.",
                Dir::Off => "Sell from 10 USDT or USDC, up to a ₦100,000,000 payout.",
            };
            let settled_limit = limit.is_some() && fl.quote_due.is_none();
            vec![
                Row::text(Span::styled(
                    "Type an amount to see a live price.",
                    Style::new().fg(TEXT),
                )),
                Row::Gap,
                Row::text(if settled_limit {
                    colored(limits, WARN)
                } else {
                    faint(limits)
                }),
            ]
        };
        Ledger(rows).render(inner, buf);
        // A price for an older amount stays visible but greyed out until the new one lands.
        if pending && quote.is_some() {
            buf.set_style(
                inner,
                Style::new().fg(FAINT).remove_modifier(Modifier::BOLD),
            );
        }
    }
}

/// A live digit count while typing, and an error once the user tried to move on.
fn digits_hint(field: &Input, len: usize, focused: bool, tried: bool) -> Option<Span<'static>> {
    if tried && field.value().len() != len {
        Some(colored(format!("Needs {len} digits"), ERR))
    } else if field.value().len() == len {
        Some(colored("✓", OK))
    } else {
        focused.then(|| faint(format!("{}/{len}", field.value().len())))
    }
}

fn party_step(f: &mut Frame, area: Rect, app: &App) {
    let fl = &app.flow;
    let focus = |x| fl.focus == x;
    let mut lines = Vec::new();
    let options = app.party_options();
    if !options.is_empty() {
        let name = fl
            .saved
            .and_then(|i| options.get(i))
            .map_or("Type new details", |(n, _)| n.as_str());
        lines.push(field(
            "Fill from",
            focus(Field::Saved),
            selector(name, focus(Field::Saved)),
            focus(Field::Saved).then(|| faint("← → saved people")),
        ));
        lines.push(blank());
    }
    let required = |empty: bool| (fl.tried && empty).then(|| colored("Required", ERR));
    let email_hint =
        (fl.tried && !fl.email.value().contains('@')).then(|| colored("Needs a valid email", ERR));
    lines.extend([
        field(
            "Full name",
            focus(Field::Name),
            input(&fl.name, focus(Field::Name), "As on their ID", INPUT_WIDTH),
            required(fl.name.value().trim().is_empty()),
        ),
        blank(),
        field(
            "Email",
            focus(Field::Email),
            input(
                &fl.email,
                focus(Field::Email),
                "name@example.com",
                INPUT_WIDTH,
            ),
            email_hint,
        ),
        blank(),
        field(
            "NIN",
            focus(Field::Nin),
            input(&fl.nin, focus(Field::Nin), "11 digits", INPUT_WIDTH),
            digits_hint(&fl.nin, 11, focus(Field::Nin), fl.tried),
        ),
        blank(),
        field(
            "BVN",
            focus(Field::Bvn),
            input(&fl.bvn, focus(Field::Bvn), "11 digits", INPUT_WIDTH),
            digits_hint(&fl.bvn, 11, focus(Field::Bvn), fl.tried),
        ),
    ]);
    let block = panel(
        if fl.dir == Dir::On {
            "Payer"
        } else {
            "Recipient"
        },
        None,
    );
    let area = fit(area, lines.len());
    let inner = block.inner(area);
    f.render_widget(block, area);
    f.render_widget(Paragraph::new(lines), inner);
}

/// Shrinks a panel to its content: borders, top padding, the lines, and one spare row.
fn fit(area: Rect, lines: usize) -> Rect {
    Rect {
        height: (lines as u16 + 4).min(area.height),
        ..area
    }
}

fn wallet_step(f: &mut Frame, area: Rect, app: &App) {
    let fl = &app.flow;
    let receive = fl
        .quote
        .as_ref()
        .map(|q| money(fl.asset, &q.destination_amount))
        .unwrap_or_default();
    let mut lines = vec![
        Line::from(vec![
            dim("Arriving  "),
            Span::styled(receive, Style::new().fg(ACCENT).bold()),
            dim(format!("  on {}", network_name(fl.network))),
        ]),
        blank(),
    ];
    if app.env() == Env::Test {
        let focused = fl.focus == Field::Outcome;
        lines.push(field(
            "Ending",
            focused,
            selector(OUTCOMES[fl.outcome].label, focused),
            focused.then(|| faint(format!("{} of {}", fl.outcome + 1, OUTCOMES.len()))),
        ));
        lines.push(blank());
        lines.push(field(
            "Address",
            false,
            vec![faint(sandbox::address(fl.outcome, fl.network))],
            None,
        ));
    } else {
        let kind = if fl.network == "SOLANA" {
            "Base58 address"
        } else {
            "0x…"
        };
        lines.push(field(
            "Address",
            true,
            input(&fl.address, true, kind, 46),
            None,
        ));
        if fl.tried {
            lines.push(under_field(vec![colored(
                format!("✕ This is not a {} address.", network_name(fl.network)),
                ERR,
            )]));
        }
    }
    let block = panel("Wallet", None);
    let area = fit(area, lines.len());
    let inner = block.inner(area);
    f.render_widget(block, area);
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn bank_step(f: &mut Frame, area: Rect, app: &App) {
    let fl = &app.flow;
    let focus = |x| fl.focus == x;
    let mut lines = Vec::new();
    if !app.config.beneficiaries.is_empty() {
        let name = fl
            .saved
            .and_then(|i| app.config.beneficiaries.get(i))
            .map_or("Type new details", |b| b.person.name.as_str());
        lines.push(field(
            "Fill from",
            focus(Field::Saved),
            selector(name, focus(Field::Saved)),
            focus(Field::Saved).then(|| faint("← → saved recipients")),
        ));
        lines.push(blank());
    }
    let bank_hint = fl.bank.as_ref().map(|_| colored("✓", OK));
    lines.push(field(
        "Bank",
        focus(Field::Bank),
        input(
            &fl.bank_query,
            focus(Field::Bank),
            "Start typing a bank",
            INPUT_WIDTH,
        ),
        bank_hint,
    ));
    if focus(Field::Bank) && fl.bank.is_none() {
        let matches = app.bank_matches();
        if app.banks_loading {
            lines.push(under_field(vec![spinner(app), dim("Loading banks…")]));
        } else if matches.is_empty() {
            lines.push(under_field(vec![faint("No bank by that name")]));
        }
        for (i, b) in matches.iter().enumerate() {
            let sel = i == fl.bank_sel;
            let name = format!(" {:<width$}", b.name, width = INPUT_WIDTH - 1);
            let row = if sel {
                vec![
                    Span::styled(name, Style::new().fg(BG).bg(ACCENT).bold()),
                    faint(format!("  {}", b.code)),
                ]
            } else {
                vec![
                    Span::styled(name, Style::new().fg(DIM)),
                    faint(format!("  {}", b.code)),
                ]
            };
            lines.push(under_field(row));
        }
    }
    lines.push(blank());
    if app.env() == Env::Test {
        let focused = focus(Field::Outcome);
        let o = &OUTCOMES[fl.outcome];
        lines.push(field("Ending", focused, selector(o.label, focused), None));
        lines.push(under_field(vec![faint(format!("Account {}", o.account))]));
    } else {
        lines.push(field(
            "Account",
            focus(Field::Account),
            input(&fl.account, focus(Field::Account), "10 digits", INPUT_WIDTH),
            digits_hint(&fl.account, 10, focus(Field::Account), false),
        ));
    }
    lines.push(blank());
    // The account status wraps inside the value column, under the fields.
    let status: Vec<Line> = if fl.resolving {
        vec![Line::from(vec![spinner(app), dim("Asking the bank…")])]
    } else if let Some(v) = &fl.resolved {
        vec![
            Line::from(vec![
                colored("✓ ", OK),
                bold(v.account_name.clone()),
                dim(format!("  ·  {}", v.bank_name)),
            ]),
            Line::from(faint("Is this the right person? Press enter to continue.")),
        ]
    } else if let Some(e) = &fl.resolve_err {
        vec![
            Line::from(colored(
                format!(
                    "✕ {}",
                    e.friendly_or(
                        "The bank could not confirm this account. \
                         Check the number, or try again in a moment."
                    )
                ),
                ERR,
            )),
            Line::from(faint(format!("Code: {}", e.code))),
        ]
    } else {
        Vec::new()
    };
    let status = Paragraph::new(status).wrap(Wrap { trim: true });
    // Borders and padding take 6 columns.
    let status_width = area.width.min(FORM_WIDTH).saturating_sub(6 + VALUE_COLUMN);
    let status_rows = status.line_count(status_width) as u16;
    let block = panel("Bank account", None);
    let area = Rect {
        height: (lines.len() as u16 + status_rows + 4).min(area.height),
        ..area
    };
    let inner = block.inner(area);
    f.render_widget(block, area);
    let [fields, rest] =
        Layout::vertical([Constraint::Length(lines.len() as u16), Constraint::Fill(1)])
            .areas(inner);
    f.render_widget(Paragraph::new(lines), fields);
    f.render_widget(
        status,
        Rect {
            x: rest.x + VALUE_COLUMN,
            width: rest.width.saturating_sub(VALUE_COLUMN),
            ..rest
        },
    );
}

fn review_step(f: &mut Frame, area: Rect, app: &App) {
    let card = ReviewCard {
        app,
        spinner: spinner(app),
    };
    let width = area.width.min(72);
    let area = Rect {
        width,
        height: card.height(width).min(area.height),
        ..area
    };
    f.render_widget(card, area);
}

/// The whole order on one card, with the button that places it.
struct ReviewCard<'a> {
    app: &'a App,
    spinner: Span<'static>,
}

impl<'a> ReviewCard<'a> {
    fn ledger(&self) -> Ledger<'a> {
        let app = self.app;
        let fl = &app.flow;
        let (src, dst) = fl.currencies();
        let mut rows = Vec::new();
        if let (Some(p), Some(q)) = (&fl.prev_quote, &fl.quote)
            && p.rate != q.rate
        {
            rows.push(Row::text(colored(
                format!(
                    "▲  New price: ₦{} → ₦{} per {}",
                    group(&p.rate),
                    group(&q.rate),
                    fl.asset
                ),
                WARN,
            )));
            rows.push(Row::Gap);
        }
        if let Some(q) = &fl.quote {
            rows.push(Row::pair("You pay", bold(money(src, &q.source_amount))));
            rows.push(Row::pair(
                "You get",
                Span::styled(
                    money(dst, &q.destination_amount),
                    Style::new().fg(ACCENT).bold(),
                ),
            ));
            rows.push(Row::pair(
                "Rate",
                format!("₦{} per {}", group(&q.rate), fl.asset),
            ));
        }
        rows.push(Row::pair("Network", network_name(fl.network)));
        rows.push(Row::Divider);
        let person = fl.person();
        match fl.dir {
            Dir::On => {
                rows.push(Row::pair("Wallet", app.destination_address()));
                rows.push(Row::pair("Payer", person.name.clone()));
            }
            Dir::Off => {
                let bank = fl.bank.as_ref().map_or(String::new(), |b| b.name.clone());
                let name = fl
                    .resolved
                    .as_ref()
                    .map_or(String::new(), |v| v.account_name.clone());
                rows.push(Row::pair("Account", name));
                rows.push(Row::pair(
                    "",
                    dim(format!("{bank}  ·  {}", app.account_number())),
                ));
                rows.push(Row::pair("Recipient", person.name.clone()));
            }
        }
        rows.push(Row::pair("Email", person.email.clone()));
        rows.push(Row::pair(
            "NIN · BVN",
            dim(format!("{}  ·  {}", mask(&person.nin), mask(&person.bvn))),
        ));
        Ledger(rows)
    }

    fn action(&self) -> Line<'static> {
        let fl = &self.app.flow;
        let live = self.app.env() == Env::Live;
        if fl.creating {
            Line::from(vec![self.spinner.clone(), dim(" Placing the order…")])
        } else if let Some(e) = &fl.create_err {
            Line::from(vec![
                colored("✕  ", ERR),
                colored(
                    e.friendly_or("Pulse could not place the order. Try again."),
                    ERR,
                ),
                faint(format!("  {}", e.code)),
            ])
        } else if live && fl.confirm_live {
            Line::from(vec![
                Span::styled(
                    "  ↵  Confirm live order  ",
                    Style::new()
                        .fg(Color::Rgb(255, 255, 255))
                        .bg(Color::Rgb(190, 18, 60))
                        .bold(),
                ),
                Span::raw("  "),
                colored("Press enter again to place it.", ERR),
            ])
        } else if live {
            Line::from(vec![
                Span::styled(
                    "  ↵  Place live order  ",
                    Style::new().fg(BG).bg(ERR).bold(),
                ),
                Span::raw("  "),
                colored("This moves real money.", ERR),
            ])
        } else {
            Line::from(vec![
                Span::styled("  ↵  Place order  ", Style::new().fg(BG).bg(ACCENT).bold()),
                Span::raw("  "),
                keycap("esc"),
                dim(" to change something"),
            ])
        }
    }

    /// Borders and padding take 4 columns of each side together and 3 rows.
    fn height(&self, width: u16) -> u16 {
        let inner = width.saturating_sub(6);
        let action = Paragraph::new(self.action())
            .wrap(Wrap { trim: false })
            .line_count(inner) as u16;
        self.ledger().height(inner) + 3 + action + 3
    }
}

impl Widget for ReviewCard<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = panel("Order", None);
        let inner = block.inner(area);
        block.render(area, buf);
        let ledger = self.ledger();
        let [rows, _, countdown, _, action] = Layout::vertical([
            Constraint::Length(ledger.height(inner.width)),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .areas(inner);
        ledger.render(rows, buf);
        Countdown {
            secs: self.app.flow.quote_secs(self.app.now),
            spinner: self.spinner.clone(),
        }
        .render(countdown, buf);
        Paragraph::new(self.action())
            .wrap(Wrap { trim: false })
            .render(action, buf);
    }
}
