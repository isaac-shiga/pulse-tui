use std::time::{Duration, Instant};

use super::*;

const POLL: Duration = Duration::from_secs(5);
const QUOTE_LIFETIME: i64 = 120;

impl App {
    pub fn tick(&mut self, now: Instant) {
        self.now = now;
        self.frame = self.frame.wrapping_add(1);
        if self.toast.as_ref().is_some_and(|t| now >= t.until) {
            self.toast = None;
        }
        match self.screen {
            Screen::Flow => self.tick_flow(now),
            Screen::Orders => {
                if let Some(t) = &mut self.orders.detail
                    && t.next_poll.is_some_and(|at| now >= at)
                {
                    t.next_poll = None;
                    let id = t.order.id.clone();
                    self.jobs.push(Job::Order {
                        target: Target::Detail,
                        id,
                    });
                }
            }
            _ => {}
        }
    }

    fn tick_flow(&mut self, now: Instant) {
        if self.flow.quote_due.is_some_and(|at| now >= at) {
            self.fire_quote();
        }
        let step = self.flow.step();
        let idle = !self.flow.quote_loading && !self.flow.creating && self.flow.quote_err.is_none();
        if matches!(step, Step::Amount | Step::Review)
            && idle
            && self.flow.quote.is_some()
            && self.flow.quote_secs(now) <= 0
        {
            self.refresh_quote(step == Step::Review);
        }
        if step == Step::Bank {
            self.maybe_resolve();
        }
        if let Some(t) = &mut self.flow.tracker
            && t.next_poll.is_some_and(|at| now >= at)
        {
            t.next_poll = None;
            let id = t.order.id.clone();
            self.jobs.push(Job::Order {
                target: Target::Flow,
                id,
            });
        }
    }

    fn maybe_resolve(&mut self) {
        let Some(bank) = &self.flow.bank else { return };
        let key = (bank.code.clone(), self.account_number());
        if key.1.len() != 10 || self.flow.resolve_key.as_ref() == Some(&key) {
            return;
        }
        let seq = self.next_seq();
        self.flow.resolve_seq = seq;
        self.flow.resolve_key = Some(key.clone());
        self.flow.resolving = true;
        self.flow.resolved = None;
        self.flow.resolve_err = None;
        self.jobs.push(Job::Resolve {
            seq,
            bank_code: key.0,
            account: key.1,
        });
    }

    pub fn on_msg(&mut self, msg: Msg) {
        let now = self.now;
        match msg {
            Msg::Quote { seq, res } => {
                if seq != self.flow.quote_seq {
                    return;
                }
                self.flow.quote_loading = false;
                let was_refresh = std::mem::take(&mut self.flow.refreshing);
                match res {
                    Ok(q) => {
                        let left = q
                            .seconds_until_expiry()
                            .filter(|s| *s > 0)
                            .unwrap_or(QUOTE_LIFETIME)
                            .min(QUOTE_LIFETIME);
                        self.flow.quote_deadline = Some(now + Duration::from_secs(left as u64));
                        self.flow.quote = Some(q);
                        self.flow.quote_err = None;
                    }
                    Err(e)
                        if was_refresh
                            && matches!(
                                e.code.as_str(),
                                "quote_not_refreshable" | "quote_not_found"
                            ) =>
                    {
                        self.fire_quote()
                    }
                    Err(e) => {
                        self.flow.quote = None;
                        self.flow.quote_err = Some(e);
                    }
                }
            }
            Msg::Banks(res) => {
                self.banks_loading = false;
                match res {
                    Ok(banks) => self.banks = banks,
                    Err(e) => self.toast(ToastKind::Error, e.friendly()),
                }
            }
            Msg::Resolved { seq, res } => {
                if seq != self.flow.resolve_seq {
                    return;
                }
                self.flow.resolving = false;
                match res {
                    Ok(v) => self.flow.resolved = Some(v),
                    Err(e) => self.flow.resolve_err = Some(e),
                }
            }
            Msg::Created(res) => {
                self.flow.creating = false;
                self.flow.confirm_live = false;
                match res {
                    Ok(order) => {
                        let first = (!order.status.is_terminal()).then_some(now + POLL);
                        self.flow.tracker = Some(Tracker::new(order, first));
                        self.go_step(4);
                    }
                    Err(e) => {
                        if matches!(e.code.as_str(), "quote_expired" | "quote_changed") {
                            self.refresh_quote(true);
                        }
                        self.flow.create_err = Some(e);
                    }
                }
            }
            Msg::Order { target, res } => {
                let tracker = match target {
                    Target::Flow => &mut self.flow.tracker,
                    Target::Detail => &mut self.orders.detail,
                };
                if let Some(t) = tracker {
                    t.update(res, now);
                }
            }
            Msg::Orders { append, res } => {
                self.orders.loading = false;
                match res {
                    Ok((items, cursor)) => {
                        if append {
                            self.orders.items.extend(items);
                        } else {
                            self.orders.items = items;
                            self.orders.sel = 0;
                        }
                        self.orders.cursor = cursor;
                    }
                    Err(e) => self.orders.error = Some(e),
                }
            }
        }
    }
}
