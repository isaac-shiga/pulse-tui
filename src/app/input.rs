use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Kind {
    #[default]
    Text,
    Digits(usize),
    Decimal,
}

#[derive(Clone, Debug, Default)]
pub struct Input {
    pub value: String,
    pub kind: Kind,
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

    pub fn set(&mut self, value: impl Into<String>) {
        self.value = value.into();
    }

    /// Returns true when the value changed.
    pub fn handle(&mut self, key: &KeyEvent) -> bool {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Delete => {
                let changed = !self.value.is_empty();
                self.value.clear();
                changed
            }
            KeyCode::Char(c)
                if !ctrl && !key.modifiers.contains(KeyModifiers::ALT) && self.accepts(c) =>
            {
                self.value.push(c);
                true
            }
            KeyCode::Backspace => self.value.pop().is_some(),
            _ => false,
        }
    }

    fn accepts(&self, c: char) -> bool {
        match self.kind {
            Kind::Text => !c.is_control(),
            Kind::Digits(max) => c.is_ascii_digit() && self.value.len() < max,
            Kind::Decimal => c.is_ascii_digit() || (c == '.' && !self.value.contains('.')),
        }
    }

    pub fn display(&self) -> String {
        if !self.secret || self.value.chars().count() <= 4 {
            return self.value.clone();
        }
        let tail: String = self
            .value
            .chars()
            .rev()
            .take(4)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        format!("{}{tail}", "•".repeat(8))
    }
}
