use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use tui_input::backend::crossterm::EventHandler;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Kind {
    #[default]
    Text,
    Digits(usize),
    Decimal,
}

/// A text field on top of `tui-input`, with a character filter per kind.
#[derive(Clone, Debug, Default)]
pub struct Input {
    inner: tui_input::Input,
    pub kind: Kind,
    /// A secret shows only its last four characters, and its cursor stays at the end.
    pub secret: bool,
}

impl Input {
    pub fn new(kind: Kind) -> Self {
        Self {
            kind,
            ..Self::default()
        }
    }

    pub fn secret(mut self) -> Self {
        self.secret = true;
        self
    }

    pub fn value(&self) -> &str {
        self.inner.value()
    }

    /// Replaces the value and puts the cursor at the end.
    pub fn set(&mut self, value: impl Into<String>) {
        self.inner = tui_input::Input::new(value.into());
    }

    /// The cursor position in characters.
    pub fn cursor(&self) -> usize {
        self.inner.cursor()
    }

    /// Returns true when the value changed.
    pub fn handle(&mut self, key: &KeyEvent) -> bool {
        if key.code == KeyCode::Delete {
            let changed = !self.value().is_empty();
            self.inner.reset();
            return changed;
        }
        let typed = !key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
        if let KeyCode::Char(c) = key.code
            && typed
            && !self.accepts(c)
        {
            return false;
        }
        if self.secret && !matches!(key.code, KeyCode::Char(_) | KeyCode::Backspace) {
            return false;
        }
        self.inner
            .handle_event(&Event::Key(*key))
            .is_some_and(|change| change.value)
    }

    fn accepts(&self, c: char) -> bool {
        match self.kind {
            Kind::Text => !c.is_control(),
            Kind::Digits(max) => c.is_ascii_digit() && self.value().chars().count() < max,
            Kind::Decimal => c.is_ascii_digit() || (c == '.' && !self.value().contains('.')),
        }
    }

    pub fn display(&self) -> String {
        let value = self.value();
        if !self.secret || value.chars().count() <= 4 {
            return value.to_string();
        }
        let tail: Vec<char> = value.chars().rev().take(4).collect();
        let tail: String = tail.into_iter().rev().collect();
        format!("{}{tail}", "•".repeat(8))
    }
}
