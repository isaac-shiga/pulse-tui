mod event;
mod input;
mod model;
pub(crate) mod sandbox;
mod update;

use std::time::{Duration, Instant};

use crate::api::Env;
use crate::config::Config;

pub(crate) use input::{Input, Kind};

pub use model::*;

pub struct App {
    pub config: Config,
    persist: bool,
    pub screen: Screen,
    pub home_sel: usize,
    pub flow: Flow,
    pub orders: OrdersView,
    pub settings: Settings,
    pub banks: Vec<crate::api::Bank>,
    pub banks_loading: bool,
    jobs: Vec<Job>,
    pub toast: Option<Toast>,
    pub frame: usize,
    pub throbber: throbber_widgets_tui::ThrobberState,
    pub now: Instant,
    pub quit: bool,
    seq: u64,
}

impl App {
    pub fn new(config: Config, persist: bool) -> Self {
        let settings = Settings::from(&config);
        Self {
            config,
            persist,
            screen: Screen::Home,
            home_sel: 0,
            flow: Flow::new(Dir::On),
            orders: OrdersView::default(),
            settings,
            banks: Vec::new(),
            banks_loading: false,
            jobs: Vec::new(),
            toast: None,
            frame: 0,
            throbber: Default::default(),
            now: Instant::now(),
            quit: false,
            seq: 0,
        }
    }

    pub fn env(&self) -> Env {
        self.config.env
    }

    pub fn key(&self) -> String {
        self.config.key(self.config.env)
    }

    fn next_seq(&mut self) -> u64 {
        self.seq += 1;
        self.seq
    }

    fn toast(&mut self, kind: ToastKind, text: impl Into<String>) {
        self.toast = Some(Toast {
            kind,
            text: text.into(),
            until: self.now + Duration::from_secs(4),
        });
    }

    fn save_config(&mut self) {
        if self.persist
            && let Err(e) = self.config.save()
        {
            self.toast(ToastKind::Error, format!("Could not save settings: {e}"));
        }
    }

    pub fn take_jobs(&mut self) -> Vec<Job> {
        std::mem::take(&mut self.jobs)
    }
}

impl Settings {
    fn from(config: &Config) -> Self {
        let mut test_key = Input::new(Kind::Text).secret();
        test_key.set(&config.test_key);
        let mut live_key = Input::new(Kind::Text).secret();
        live_key.set(&config.live_key);
        Self {
            env: config.env,
            test_key,
            live_key,
            focus: 0,
        }
    }
}
