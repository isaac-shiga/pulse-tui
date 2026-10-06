use std::time::{Duration, Instant};

use super::input::{Input, Kind};
use crate::api::{
    ApiError, Bank, CreateOrderRequest, Env, Order, Quote, QuoteRequest, Status, VerifiedAccount,
};
use crate::config::Person;

pub const ASSETS: [&str; 2] = ["USDT", "USDC"];
const NETWORKS: [&str; 8] = [
    "BASE", "POLYGON", "ARBITRUM", "OPTIMISM", "ETHEREUM", "CELO", "PLASMA", "SOLANA",
];
const POLL: Duration = Duration::from_secs(5);

/// Pulse order limits. A buy is ₦15,000 to ₦100,000,000. A sell is at
/// least 10 USDT or USDC, with a payout of up to ₦100,000,000.
const MIN_BUY_NGN: f64 = 15_000.0;
const MAX_NGN: f64 = 100_000_000.0;
const MIN_SELL_COIN: f64 = 10.0;

/// An amount outside the Pulse limits, as a currency and an amount.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Limit {
    Min(&'static str, &'static str),
    Max(&'static str, &'static str),
}

/// `PLASMA` carries USDT only.
pub fn networks_for(asset: &str) -> Vec<&'static str> {
    NETWORKS
        .iter()
        .copied()
        .filter(|n| asset == "USDT" || *n != "PLASMA")
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Home,
    Flow,
    Orders,
    Settings,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    On,
    Off,
}

impl Dir {
    pub fn title(self) -> &'static str {
        match self {
            Dir::On => "Buy stablecoins",
            Dir::Off => "Sell stablecoins",
        }
    }

    pub fn steps(self) -> [Step; 5] {
        match self {
            Dir::On => [
                Step::Amount,
                Step::Party,
                Step::Wallet,
                Step::Review,
                Step::Track,
            ],
            Dir::Off => [
                Step::Amount,
                Step::Bank,
                Step::Party,
                Step::Review,
                Step::Track,
            ],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Amount,
    Party,
    Wallet,
    Bank,
    Review,
    Track,
}

impl Step {
    pub fn label(self, dir: Dir) -> &'static str {
        match self {
            Step::Amount => "Amount",
            Step::Party if dir == Dir::On => "Your details",
            Step::Party => "Recipient",
            Step::Wallet => "Wallet",
            Step::Bank => "Bank account",
            Step::Review => "Review",
            Step::Track => "Track",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Asset,
    Network,
    Amount,
    Saved,
    Name,
    Email,
    Nin,
    Bvn,
    Outcome,
    Address,
    Bank,
    Account,
}

impl Field {
    /// A free text field, where the arrow keys move the cursor.
    pub fn is_text(self) -> bool {
        matches!(
            self,
            Field::Name
                | Field::Email
                | Field::Nin
                | Field::Bvn
                | Field::Address
                | Field::Bank
                | Field::Account
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Done,
    Current,
    Todo,
    Bad,
}

/// Polls one order until it reaches a terminal status.
#[derive(Clone, Debug)]
pub struct Tracker {
    pub order: Order,
    pub seen: Vec<Status>,
    pub next_poll: Option<Instant>,
    pub error: Option<ApiError>,
    /// The value `c` copies.
    pub pick: Copyable,
    /// False while the order is the summary from the order list, before
    /// the first full fetch.
    pub fresh: bool,
}

/// A value on the order screen the user can copy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Copyable {
    Amount,
    /// The account number for a buy, or the deposit address for a sell.
    Funding,
    Reference,
    OrderId,
}

impl Copyable {
    pub fn label(self, order: &Order) -> &'static str {
        match self {
            Copyable::Amount => "amount",
            Copyable::Funding if order.is_onramp() => "account number",
            Copyable::Funding => "deposit address",
            Copyable::Reference => "reference",
            Copyable::OrderId => "order ID",
        }
    }
}

impl Tracker {
    pub(crate) fn new(order: Order, first_poll: Option<Instant>) -> Self {
        // Start on what the user most likely needs: where to pay, or the
        // order ID for support once the order went wrong.
        let pick = match order.status {
            Status::Failed | Status::Expired => Copyable::OrderId,
            _ if funding_value(&order).is_some() => Copyable::Funding,
            _ => Copyable::Amount,
        };
        Self {
            seen: vec![order.status],
            order,
            next_poll: first_poll,
            error: None,
            pick,
            fresh: true,
        }
    }

    /// A tracker for an order from the order list. The caller fetches the
    /// full order.
    pub(crate) fn from_list(order: Order) -> Self {
        Self {
            fresh: false,
            ..Self::new(order, None)
        }
    }

    /// The value of a copy target, when the order has one.
    pub fn value(&self, c: Copyable) -> Option<String> {
        let o = &self.order;
        let value = match c {
            Copyable::Amount => match &o.funding_account {
                Some(fa) if o.is_onramp() => fa.amount.clone().or(Some(o.source.amount.clone())),
                _ => Some(o.source.amount.clone()),
            },
            Copyable::Funding => funding_value(o),
            Copyable::Reference => Some(o.reference.clone()),
            Copyable::OrderId => Some(o.id.clone()),
        };
        value.filter(|v| !v.is_empty())
    }

    /// The copy targets that have a value, top to bottom on screen.
    pub fn targets(&self) -> Vec<Copyable> {
        [
            Copyable::Amount,
            Copyable::Funding,
            Copyable::Reference,
            Copyable::OrderId,
        ]
        .into_iter()
        .filter(|c| self.value(*c).is_some())
        .collect()
    }

    pub(crate) fn move_pick(&mut self, step: isize) {
        let targets = self.targets();
        let i = targets.iter().position(|c| *c == self.pick).unwrap_or(0);
        let next = (i as isize + step).clamp(0, targets.len() as isize - 1);
        if let Some(c) = targets.get(next as usize) {
            self.pick = *c;
        }
    }

    pub(crate) fn update(&mut self, res: Result<Order, ApiError>, now: Instant) {
        match res {
            Ok(order) => {
                // Jump to the account or address the moment Pulse issues it.
                if funding_value(&self.order).is_none() && funding_value(&order).is_some() {
                    self.pick = Copyable::Funding;
                }
                if !self.seen.contains(&order.status) {
                    self.seen.push(order.status);
                }
                self.next_poll = (!order.status.is_terminal()).then_some(now + POLL);
                self.order = order;
                self.error = None;
                self.fresh = true;
            }
            Err(e) => {
                let wait = if e.code == "rate_limited" { 60 } else { 10 };
                self.next_poll = Some(now + Duration::from_secs(wait));
                self.error = Some(e);
            }
        }
    }

    /// The states to draw. An order can fail from any state, so a failure
    /// shows the states the order reached first.
    pub fn timeline(&self) -> Vec<(Status, Mark)> {
        use Status::*;
        let processed = self.seen.contains(&Processing);
        let paid = processed || self.seen.contains(&AwaitingPayment);
        let reached = |s, yes: bool| yes.then_some((s, Mark::Done));
        match self.order.status {
            AwaitingPayment => vec![
                (AwaitingPayment, Mark::Current),
                (Processing, Mark::Todo),
                (Completed, Mark::Todo),
            ],
            Processing => vec![
                (AwaitingPayment, Mark::Done),
                (Processing, Mark::Current),
                (Completed, Mark::Todo),
            ],
            Completed => vec![
                (AwaitingPayment, Mark::Done),
                (Processing, Mark::Done),
                (Completed, Mark::Done),
            ],
            Expired => vec![(AwaitingPayment, Mark::Done), (Expired, Mark::Bad)],
            Failed => [
                reached(AwaitingPayment, paid),
                reached(Processing, processed),
                Some((Failed, Mark::Bad)),
            ]
            .into_iter()
            .flatten()
            .collect(),
            Unknown => vec![(Unknown, Mark::Current)],
        }
    }
}

pub struct Flow {
    pub dir: Dir,
    pub step_idx: usize,
    pub focus: Field,
    pub tried: bool,
    pub asset: &'static str,
    pub network: &'static str,
    pub fix_ngn: bool,
    pub amount: Input,
    pub quote: Option<Quote>,
    pub prev_quote: Option<Quote>,
    pub quote_err: Option<ApiError>,
    pub quote_loading: bool,
    pub quote_due: Option<Instant>,
    pub quote_deadline: Option<Instant>,
    pub(super) quote_seq: u64,
    pub(super) refreshing: bool,
    pub saved: Option<usize>,
    pub name: Input,
    pub email: Input,
    pub nin: Input,
    pub bvn: Input,
    pub outcome: usize,
    pub address: Input,
    pub bank_query: Input,
    pub bank_sel: usize,
    pub bank: Option<Bank>,
    pub account: Input,
    pub resolved: Option<VerifiedAccount>,
    pub resolve_err: Option<ApiError>,
    pub resolving: bool,
    pub(super) resolve_key: Option<(String, String)>,
    pub(super) resolve_seq: u64,
    pub reference: String,
    pub confirm_live: bool,
    pub creating: bool,
    pub create_err: Option<ApiError>,
    pub tracker: Option<Tracker>,
}

impl Flow {
    pub fn new(dir: Dir) -> Self {
        Self {
            dir,
            step_idx: 0,
            focus: Field::Amount,
            tried: false,
            asset: "USDT",
            network: "BASE",
            fix_ngn: dir == Dir::On,
            amount: Input::new(Kind::Decimal),
            quote: None,
            prev_quote: None,
            quote_err: None,
            quote_loading: false,
            quote_due: None,
            quote_deadline: None,
            quote_seq: 0,
            refreshing: false,
            saved: None,
            name: Input::new(Kind::Text),
            email: Input::new(Kind::Text),
            nin: Input::new(Kind::Digits(11)),
            bvn: Input::new(Kind::Digits(11)),
            outcome: 0,
            address: Input::new(Kind::Text),
            bank_query: Input::new(Kind::Text),
            bank_sel: 0,
            bank: None,
            account: Input::new(Kind::Digits(10)),
            resolved: None,
            resolve_err: None,
            resolving: false,
            resolve_key: None,
            resolve_seq: 0,
            reference: String::new(),
            confirm_live: false,
            creating: false,
            create_err: None,
            tracker: None,
        }
    }

    pub fn step(&self) -> Step {
        self.dir.steps()[self.step_idx]
    }

    pub fn currencies(&self) -> (&'static str, &'static str) {
        match self.dir {
            Dir::On => ("NGN", self.asset),
            Dir::Off => (self.asset, "NGN"),
        }
    }

    pub fn fixed_is_source(&self) -> bool {
        (self.dir == Dir::On) == self.fix_ngn
    }

    /// The label and currency of the amount the user types.
    pub fn typed_side(&self) -> (&'static str, &'static str) {
        let label = if self.fixed_is_source() {
            "You pay"
        } else {
            "You get"
        };
        (label, if self.fix_ngn { "NGN" } else { self.asset })
    }

    pub fn quote_secs(&self, now: Instant) -> i64 {
        self.quote_deadline
            .map_or(0, |d| d.saturating_duration_since(now).as_secs() as i64)
    }

    fn typed_amount(&self) -> Option<f64> {
        self.amount.value().trim().parse().ok()
    }

    /// The limit the typed amount breaks. The app checks only limits in the
    /// currency the user types. Pulse checks the rest, because they depend
    /// on the rate.
    pub fn amount_limit(&self) -> Option<Limit> {
        let amount = self.typed_amount()?;
        let (_, unit) = self.typed_side();
        match (self.dir, self.fix_ngn) {
            (Dir::On, true) if amount < MIN_BUY_NGN => Some(Limit::Min(unit, "15000")),
            (Dir::Off, false) if amount < MIN_SELL_COIN => Some(Limit::Min(unit, "10")),
            (_, true) if amount > MAX_NGN => Some(Limit::Max(unit, "100000000")),
            _ => None,
        }
    }

    pub(super) fn quote_request(&self) -> Option<QuoteRequest> {
        let amount = self.amount.value().trim();
        if self.typed_amount().is_none_or(|a| a <= 0.0) || self.amount_limit().is_some() {
            return None;
        }
        let (source, destination) = self.currencies();
        let fixed_source = self.fixed_is_source();
        Some(QuoteRequest {
            source_currency: source.into(),
            destination_currency: destination.into(),
            network: self.network.into(),
            source_amount: fixed_source.then(|| amount.to_string()),
            destination_amount: (!fixed_source).then(|| amount.to_string()),
        })
    }

    pub(super) fn first_invalid_party(&self) -> Option<Field> {
        if self.name.value().trim().is_empty() {
            Some(Field::Name)
        } else if !self.email.value().contains('@') {
            Some(Field::Email)
        } else if self.nin.value().len() != 11 {
            Some(Field::Nin)
        } else if self.bvn.value().len() != 11 {
            Some(Field::Bvn)
        } else {
            None
        }
    }

    pub fn person(&self) -> Person {
        Person {
            name: self.name.value().trim().into(),
            email: self.email.value().trim().into(),
            nin: self.nin.value().to_string(),
            bvn: self.bvn.value().to_string(),
        }
    }

    pub(super) fn fill_person(&mut self, p: &Person) {
        self.name.set(&p.name);
        self.email.set(&p.email);
        self.nin.set(&p.nin);
        self.bvn.set(&p.bvn);
    }
}

fn funding_value(o: &Order) -> Option<String> {
    let fa = o.funding_account.as_ref()?;
    if o.is_onramp() {
        fa.account_number.clone()
    } else {
        fa.deposit_address.clone()
    }
}

pub fn address_valid(network: &str, address: &str) -> bool {
    if network == "SOLANA" {
        const BASE58: &str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
        (32..=44).contains(&address.len()) && address.chars().all(|c| BASE58.contains(c))
    } else {
        address.len() == 42
            && address.starts_with("0x")
            && address[2..].chars().all(|c| c.is_ascii_hexdigit())
    }
}

#[derive(Default)]
pub struct OrdersView {
    pub items: Vec<Order>,
    pub sel: usize,
    pub cursor: Option<String>,
    pub filter: Option<&'static str>,
    pub loading: bool,
    pub error: Option<ApiError>,
    pub detail: Option<Tracker>,
}

pub struct Settings {
    pub env: Env,
    pub test_key: Input,
    pub live_key: Input,
    pub focus: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Success,
    Error,
}

pub struct Toast {
    pub kind: ToastKind,
    pub text: String,
    pub(super) until: Instant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Flow,
    Detail,
}

#[derive(Debug, PartialEq)]
pub enum Job {
    Quote {
        seq: u64,
        req: QuoteRequest,
    },
    Refresh {
        seq: u64,
        id: String,
    },
    Banks,
    Resolve {
        seq: u64,
        bank_code: String,
        account: String,
    },
    Create(CreateOrderRequest),
    Order {
        target: Target,
        id: String,
    },
    Orders {
        kind: Option<&'static str>,
        cursor: Option<String>,
    },
    Copy(String),
}

pub enum Msg {
    Quote {
        seq: u64,
        res: Result<Quote, ApiError>,
    },
    Banks(Result<Vec<Bank>, ApiError>),
    Resolved {
        seq: u64,
        res: Result<VerifiedAccount, ApiError>,
    },
    Created(Result<Order, ApiError>),
    Order {
        target: Target,
        /// The order asked for, so a late answer for another order is dropped.
        id: String,
        res: Result<Order, ApiError>,
    },
    Orders {
        append: bool,
        res: Result<(Vec<Order>, Option<String>), ApiError>,
    },
}
