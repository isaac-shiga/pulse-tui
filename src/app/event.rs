use std::time::Duration;

use super::sandbox::{self, OUTCOMES};
use super::*;
use crate::api::{
    Bank, BankAccountRequest, BeneficiaryRequest, CreateOrderRequest, DestinationRequest, Env,
    OfframpRequest, OnrampRequest, PartyRequest,
};
use crate::config::{Beneficiary, Person};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

const DEBOUNCE: Duration = Duration::from_millis(400);

impl App {
    pub fn on_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.quit = true;
            return;
        }
        match self.screen {
            Screen::Home => self.home_key(key),
            Screen::Flow => self.flow_key(key),
            Screen::Orders => self.orders_key(key),
            Screen::Settings => self.settings_key(key),
        }
    }

    fn home_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.home_sel = self.home_sel.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => self.home_sel = (self.home_sel + 1).min(3),
            KeyCode::Enter => self.open(self.home_sel),
            KeyCode::Char(c @ '1'..='4') => {
                self.home_sel = c as usize - '1' as usize;
                self.open(self.home_sel);
            }
            KeyCode::Char('e') => {
                self.config.env = self.config.env.toggle();
                self.save_config();
                self.toast(
                    ToastKind::Info,
                    format!("Switched to {}", self.env().label().to_lowercase()),
                );
            }
            KeyCode::Char('q') => self.quit = true,
            _ => {}
        }
    }

    fn open(&mut self, item: usize) {
        match item {
            0 => self.start_flow(Dir::On),
            1 => self.start_flow(Dir::Off),
            2 => self.open_orders(),
            _ => self.open_settings(),
        }
    }

    fn require_key(&mut self) -> bool {
        if self.key().is_empty() {
            self.toast(
                ToastKind::Error,
                format!(
                    "Add your {} API key first.",
                    self.env().label().to_lowercase()
                ),
            );
            self.open_settings();
            return false;
        }
        true
    }

    fn start_flow(&mut self, dir: Dir) {
        if !self.require_key() {
            return;
        }
        self.flow = Flow::new(dir);
        self.screen = Screen::Flow;
        if dir == Dir::Off && self.banks.is_empty() && !self.banks_loading {
            self.banks_loading = true;
            self.jobs.push(Job::Banks);
        }
    }

    fn open_orders(&mut self) {
        if !self.require_key() {
            return;
        }
        self.screen = Screen::Orders;
        self.orders.detail = None;
        self.load_orders(None);
    }

    fn load_orders(&mut self, cursor: Option<String>) {
        self.orders.loading = true;
        self.orders.error = None;
        self.jobs.push(Job::Orders {
            kind: self.orders.filter,
            cursor,
        });
    }

    fn open_settings(&mut self) {
        self.settings = Settings::from(&self.config);
        self.screen = Screen::Settings;
    }

    pub fn fields(&self) -> Vec<Field> {
        let test = self.env() == Env::Test;
        match self.flow.step() {
            Step::Amount => vec![Field::Asset, Field::Network, Field::Amount],
            Step::Party => {
                let mut v = Vec::new();
                if !self.party_options().is_empty() {
                    v.push(Field::Saved);
                }
                v.extend([Field::Name, Field::Email, Field::Nin, Field::Bvn]);
                v
            }
            Step::Wallet => vec![if test { Field::Outcome } else { Field::Address }],
            Step::Bank => {
                let mut v = Vec::new();
                if !self.config.beneficiaries.is_empty() {
                    v.push(Field::Saved);
                }
                v.push(Field::Bank);
                v.push(if test { Field::Outcome } else { Field::Account });
                v
            }
            Step::Review | Step::Track => Vec::new(),
        }
    }

    /// Saved payers, plus the sandbox identity in test mode. Off-ramp
    /// beneficiaries are picked on the bank step, with their bank account.
    pub fn party_options(&self) -> Vec<(String, Person)> {
        let mut v: Vec<(String, Person)> = match self.flow.dir {
            Dir::On => self
                .config
                .payers
                .iter()
                .map(|p| (p.name.clone(), p.clone()))
                .collect(),
            Dir::Off => Vec::new(),
        };
        if self.env() == Env::Test {
            v.push(("Sandbox identity".into(), sandbox::identity()));
        }
        v
    }

    pub fn bank_matches(&self) -> Vec<&Bank> {
        let q = self.flow.bank_query.value().to_lowercase();
        self.banks
            .iter()
            .filter(|b| b.name.to_lowercase().contains(&q))
            .take(6)
            .collect()
    }

    pub fn destination_address(&self) -> String {
        if self.env() == Env::Test {
            sandbox::address(self.flow.outcome, self.flow.network).to_string()
        } else {
            self.flow.address.value().trim().to_string()
        }
    }

    pub fn account_number(&self) -> String {
        if self.env() == Env::Test {
            OUTCOMES[self.flow.outcome].account.to_string()
        } else {
            self.flow.account.value().to_string()
        }
    }

    fn flow_key(&mut self, key: KeyEvent) {
        let step = self.flow.step();
        if key.code == KeyCode::Esc {
            if step == Step::Track || self.flow.step_idx == 0 {
                self.screen = Screen::Home;
            } else {
                self.go_step(self.flow.step_idx - 1);
            }
            return;
        }
        match step {
            Step::Review => {
                if key.code == KeyCode::Enter {
                    self.place_order();
                }
                return;
            }
            Step::Track => {
                self.track_key(key);
                return;
            }
            _ => {}
        }

        let fields = self.fields();
        let pos = fields
            .iter()
            .position(|f| *f == self.flow.focus)
            .unwrap_or(0);
        let focus = fields.get(pos).copied();
        let bank_list = focus == Some(Field::Bank)
            && self.flow.bank.is_none()
            && !self.bank_matches().is_empty();
        match key.code {
            KeyCode::Up if bank_list => self.flow.bank_sel = self.flow.bank_sel.saturating_sub(1),
            KeyCode::Down if bank_list => {
                self.flow.bank_sel = (self.flow.bank_sel + 1).min(self.bank_matches().len() - 1);
            }
            KeyCode::Tab | KeyCode::Enter if bank_list => {
                self.pick_bank();
                self.focus_at(pos + 1);
            }
            KeyCode::Tab | KeyCode::Down => self.focus_at(pos + 1),
            KeyCode::BackTab | KeyCode::Up => self.focus_at(pos.saturating_sub(1)),
            KeyCode::Left | KeyCode::Right => match focus {
                // Text fields move their cursor. The amount swaps sides instead.
                Some(f) if f.is_text() => self.edit(f, &key),
                Some(f) => self.cycle(f, if key.code == KeyCode::Right { 1 } else { -1 }),
                None => {}
            },
            KeyCode::Enter => self.advance(),
            _ => {
                if let Some(f) = focus {
                    self.edit(f, &key);
                }
            }
        }
    }

    fn focus_at(&mut self, i: usize) {
        if let Some(f) = self.fields().get(i) {
            self.flow.focus = *f;
        }
    }

    fn cycle(&mut self, field: Field, d: isize) {
        let step = |i: usize, len: usize| (i as isize + d).rem_euclid(len as isize) as usize;
        match field {
            Field::Asset => {
                let i = ASSETS
                    .iter()
                    .position(|a| *a == self.flow.asset)
                    .unwrap_or(0);
                self.flow.asset = ASSETS[step(i, ASSETS.len())];
                if !networks_for(self.flow.asset).contains(&self.flow.network) {
                    self.flow.network = "BASE";
                }
                self.flow.quote_due = Some(self.now + DEBOUNCE);
            }
            Field::Network => {
                let list = networks_for(self.flow.asset);
                let i = list
                    .iter()
                    .position(|n| *n == self.flow.network)
                    .unwrap_or(0);
                self.flow.network = list[step(i, list.len())];
                self.flow.quote_due = Some(self.now + DEBOUNCE);
            }
            Field::Amount => {
                self.flow.fix_ngn = !self.flow.fix_ngn;
                if let Some(q) = &self.flow.quote {
                    let other = if self.flow.fixed_is_source() {
                        &q.source_amount
                    } else {
                        &q.destination_amount
                    };
                    self.flow.amount.set(other.clone());
                }
                self.flow.quote_due = Some(self.now + DEBOUNCE);
            }
            Field::Saved if self.flow.step() == Step::Party => {
                let options = self.party_options();
                let i = self
                    .flow
                    .saved
                    .map_or(if d > 0 { 0 } else { options.len() - 1 }, |i| {
                        step(i, options.len())
                    });
                self.flow.saved = Some(i);
                self.flow.fill_person(&options[i].1);
            }
            Field::Saved => {
                let list = &self.config.beneficiaries;
                let i = self
                    .flow
                    .saved
                    .map_or(if d > 0 { 0 } else { list.len() - 1 }, |i| {
                        step(i, list.len())
                    });
                let b = list[i].clone();
                self.flow.saved = Some(i);
                self.flow.bank = Some(Bank {
                    name: b.bank_name.clone(),
                    code: b.bank_code.clone(),
                });
                self.flow.bank_query.set(b.bank_name);
                match OUTCOMES.iter().position(|o| o.account == b.account_number) {
                    Some(o) if self.env() == Env::Test => self.flow.outcome = o,
                    _ => self.flow.account.set(b.account_number),
                }
                self.flow.fill_person(&b.person);
                self.flow.resolved = None;
            }
            Field::Outcome => {
                self.flow.outcome = step(self.flow.outcome, OUTCOMES.len());
                self.flow.resolved = None;
            }
            _ => {}
        }
    }

    fn edit(&mut self, field: Field, key: &KeyEvent) {
        let now = self.now;
        let f = &mut self.flow;
        let input = match field {
            Field::Amount => &mut f.amount,
            Field::Name => &mut f.name,
            Field::Email => &mut f.email,
            Field::Nin => &mut f.nin,
            Field::Bvn => &mut f.bvn,
            Field::Address => &mut f.address,
            Field::Bank => &mut f.bank_query,
            Field::Account => &mut f.account,
            _ => return,
        };
        if !input.handle(key) {
            return;
        }
        match field {
            Field::Amount if f.amount.value().is_empty() => {
                f.quote = None;
                f.quote_err = None;
                f.quote_due = None;
                f.quote_loading = false;
                f.quote_seq = 0;
            }
            Field::Amount => f.quote_due = Some(now + DEBOUNCE),
            Field::Bank => {
                f.bank = None;
                f.bank_sel = 0;
                f.resolved = None;
                f.saved = None;
            }
            Field::Account => f.resolved = None,
            _ => f.saved = None,
        }
    }

    fn pick_bank(&mut self) {
        if let Some(bank) = self
            .bank_matches()
            .get(self.flow.bank_sel)
            .map(|b| (*b).clone())
        {
            self.flow.bank_query.set(&bank.name);
            self.flow.bank = Some(bank);
        }
    }

    fn advance(&mut self) {
        match self.flow.step() {
            Step::Amount => {
                if self.flow.amount_limit().is_some() {
                    return self.toast(ToastKind::Error, "Change the amount to fit the limit.");
                }
                if !self.flow.priceable() {
                    return self.toast(ToastKind::Error, "Type an amount to get a quote.");
                }
                if self.flow.quote_loading || self.flow.quote_due.is_some() {
                    return self.toast(ToastKind::Info, "Getting a quote…");
                }
                if self.flow.quote.is_none() {
                    return self.toast(ToastKind::Error, "Type an amount to get a quote.");
                }
            }
            Step::Party => {
                if let Some(bad) = self.flow.first_invalid_party() {
                    self.flow.tried = true;
                    self.flow.focus = bad;
                    return;
                }
            }
            Step::Wallet => {
                if self.env() == Env::Live
                    && !address_valid(self.flow.network, self.flow.address.value().trim())
                {
                    self.flow.tried = true;
                    return;
                }
            }
            Step::Bank => {
                let Some(v) = &self.flow.resolved else {
                    let text = if self.flow.resolving {
                        "Checking the account…"
                    } else {
                        "Pick a bank and an account the bank confirms."
                    };
                    return self.toast(ToastKind::Error, text);
                };
                if self.flow.name.value().is_empty() {
                    let name = v.account_name.clone();
                    self.flow.name.set(name);
                }
            }
            Step::Review | Step::Track => return,
        }
        self.go_step(self.flow.step_idx + 1);
    }

    pub(super) fn go_step(&mut self, i: usize) {
        self.flow.step_idx = i;
        self.flow.tried = false;
        if let Some(f) = self.fields().first() {
            self.flow.focus = *f;
        }
        if self.flow.step() == Step::Review {
            self.flow.reference = format!("tui-{}", uuid::Uuid::new_v4().simple());
            self.flow.create_err = None;
            self.flow.confirm_live = false;
            self.flow.prev_quote = None;
            // Review is where the price locks, so it gets a full quote lifetime.
            self.refresh_quote(true);
        }
    }

    pub(super) fn fire_quote(&mut self) {
        self.flow.quote_due = None;
        self.flow.refreshing = false;
        let Some(req) = self.flow.quote_request() else {
            // No amount to price, so no old price may stay on screen.
            self.flow.quote = None;
            self.flow.quote_err = None;
            self.flow.quote_loading = false;
            self.flow.quote_seq = 0;
            return;
        };
        let seq = self.next_seq();
        self.flow.quote_seq = seq;
        self.flow.quote_loading = true;
        self.jobs.push(Job::Quote { seq, req });
    }

    pub(super) fn refresh_quote(&mut self, keep_prev: bool) {
        let Some(q) = self.flow.quote.clone() else {
            return;
        };
        let seq = self.next_seq();
        self.flow.quote_seq = seq;
        self.flow.quote_loading = true;
        self.flow.refreshing = true;
        if keep_prev {
            self.flow.prev_quote = Some(q.clone());
        }
        self.jobs.push(Job::Refresh { seq, id: q.id });
    }

    fn place_order(&mut self) {
        if self.flow.creating || self.flow.quote_loading {
            return;
        }
        let Some(q) = self.flow.quote.clone() else {
            return;
        };
        if self.flow.quote_secs(self.now) <= 0 {
            return self.refresh_quote(true);
        }
        if self.env() == Env::Live && !self.flow.confirm_live {
            self.flow.confirm_live = true;
            return;
        }
        let f = &self.flow;
        let party = PartyRequest {
            name: f.name.value().trim().to_string(),
            email: f.email.value().trim().to_string(),
            nin: f.nin.value().to_string(),
            bvn: f.bvn.value().to_string(),
        };
        let request = match f.dir {
            Dir::On => CreateOrderRequest::Onramp(OnrampRequest {
                reference: f.reference.clone(),
                quote_id: q.id,
                payer: party,
                destination: DestinationRequest {
                    address: self.destination_address(),
                },
            }),
            Dir::Off => {
                let bank_code = f.bank.as_ref().map(|b| b.code.clone()).unwrap_or_default();
                CreateOrderRequest::Offramp(OfframpRequest {
                    reference: f.reference.clone(),
                    quote_id: q.id,
                    beneficiary: BeneficiaryRequest {
                        name: party.name,
                        email: party.email,
                        nin: party.nin,
                        bvn: party.bvn,
                        bank_account: BankAccountRequest {
                            bank_code,
                            account_number: self.account_number(),
                        },
                    },
                })
            }
        };
        self.flow.creating = true;
        self.flow.create_err = None;
        self.jobs.push(Job::Create(request));
    }

    fn track_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(t) = &mut self.flow.tracker {
                    t.move_pick(-1);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(t) = &mut self.flow.tracker {
                    t.move_pick(1);
                }
            }
            KeyCode::Char('c') => {
                if let Some(t) = self.flow.tracker.clone() {
                    self.copy(&t);
                }
            }
            KeyCode::Char('s') if !self.profile_saved() => self.save_profile(),
            KeyCode::Char('n') => self.start_flow(self.flow.dir),
            KeyCode::Enter => self.screen = Screen::Home,
            _ => {}
        }
    }

    fn copy(&mut self, t: &Tracker) {
        let label = t.pick.label(&t.order);
        match t.value(t.pick) {
            Some(v) => {
                self.jobs.push(Job::Copy(v));
                self.toast(ToastKind::Success, format!("Copied the {label}"));
            }
            None => self.toast(ToastKind::Info, format!("No {label} yet")),
        }
    }

    pub fn profile_saved(&self) -> bool {
        let person = self.flow.person();
        match self.flow.dir {
            Dir::On => self.config.payers.contains(&person),
            Dir::Off => self
                .config
                .beneficiaries
                .iter()
                .any(|b| b.person == person && b.account_number == self.account_number()),
        }
    }

    fn save_profile(&mut self) {
        let person = self.flow.person();
        let name = person.name.clone();
        match self.flow.dir {
            Dir::On => self.config.payers.push(person),
            Dir::Off => {
                let bank = self.flow.bank.clone().unwrap_or(Bank {
                    name: String::new(),
                    code: String::new(),
                });
                let account_name = self
                    .flow
                    .resolved
                    .as_ref()
                    .map(|v| v.account_name.clone())
                    .unwrap_or_default();
                let account_number = self.account_number();
                self.config.beneficiaries.push(Beneficiary {
                    person,
                    bank_code: bank.code,
                    bank_name: bank.name,
                    account_number,
                    account_name,
                });
            }
        }
        self.save_config();
        self.toast(ToastKind::Success, format!("Saved {name}"));
    }

    fn orders_key(&mut self, key: KeyEvent) {
        if let Some(t) = &mut self.orders.detail {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => t.move_pick(-1),
                KeyCode::Down | KeyCode::Char('j') => t.move_pick(1),
                KeyCode::Esc | KeyCode::Enter => self.orders.detail = None,
                KeyCode::Char('c') => {
                    let t = t.clone();
                    self.copy(&t);
                }
                _ => {}
            }
            return;
        }
        match key.code {
            KeyCode::Esc => self.screen = Screen::Home,
            KeyCode::Up | KeyCode::Char('k') => self.orders.sel = self.orders.sel.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => {
                self.orders.sel =
                    (self.orders.sel + 1).min(self.orders.items.len().saturating_sub(1));
            }
            KeyCode::Enter => {
                if let Some(order) = self.orders.items.get(self.orders.sel) {
                    // Fetch on the key press, not on the next tick.
                    self.jobs.push(Job::Order {
                        target: Target::Detail,
                        id: order.id.clone(),
                    });
                    self.orders.detail = Some(Tracker::from_list(order.clone()));
                }
            }
            KeyCode::Char('r') => self.load_orders(None),
            KeyCode::Char('t') => {
                self.orders.filter = match self.orders.filter {
                    None => Some("onramp"),
                    Some("onramp") => Some("offramp"),
                    _ => None,
                };
                self.load_orders(None);
            }
            KeyCode::Char('m') => {
                if let Some(cursor) = self.orders.cursor.clone() {
                    self.load_orders(Some(cursor));
                }
            }
            _ => {}
        }
    }

    fn settings_key(&mut self, key: KeyEvent) {
        let s = &mut self.settings;
        match key.code {
            KeyCode::Esc => self.screen = Screen::Home,
            KeyCode::Tab | KeyCode::Down => s.focus = (s.focus + 1).min(2),
            KeyCode::BackTab | KeyCode::Up => s.focus = s.focus.saturating_sub(1),
            KeyCode::Left | KeyCode::Right if s.focus == 0 => s.env = s.env.toggle(),
            KeyCode::Enter => {
                self.config.env = s.env;
                self.config.test_key = s.test_key.value().trim().to_string();
                self.config.live_key = s.live_key.value().trim().to_string();
                self.save_config();
                self.toast(ToastKind::Success, "Settings saved");
                self.screen = Screen::Home;
            }
            _ => {
                match s.focus {
                    1 => s.test_key.handle(&key),
                    2 => s.live_key.handle(&key),
                    _ => false,
                };
            }
        }
    }
}
