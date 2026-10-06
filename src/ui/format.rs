use chrono::{DateTime, Local};

/// Adds thousands separators and keeps every decimal the API sent.
pub(crate) fn group(amount: &str) -> String {
    let (int, frac) = amount
        .split_once('.')
        .map_or((amount, None), |(i, f)| (i, Some(f)));
    let digits = int.trim_start_matches('-');
    let mut out = String::new();
    if int.starts_with('-') {
        out.push('-');
    }
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if let Some(frac) = frac {
        out.push('.');
        out.push_str(frac);
    }
    out
}

pub(crate) fn money(currency: &str, amount: &str) -> String {
    if currency == "NGN" {
        format!("₦{}", group(amount))
    } else {
        format!("{} {currency}", group(amount))
    }
}

pub(crate) fn clock(secs: i64) -> String {
    let s = secs.max(0);
    format!("{}:{:02}", s / 60, s % 60)
}

pub(crate) fn short_time(timestamp: &str) -> String {
    DateTime::parse_from_rfc3339(timestamp)
        .map(|t| t.with_timezone(&Local).format("%d %b %H:%M").to_string())
        .unwrap_or_default()
}

pub(crate) fn mask(value: &str) -> String {
    if value.chars().count() <= 4 {
        value.to_string()
    } else {
        let tail: String = value
            .chars()
            .rev()
            .take(4)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        format!("••••{tail}")
    }
}

/// Turns an API network code such as `ARBITRUM` into `Arbitrum`.
pub(crate) fn network_name(code: &str) -> String {
    let mut chars = code.chars();
    match chars.next() {
        Some(first) => first
            .to_uppercase()
            .chain(chars.flat_map(char::to_lowercase))
            .collect(),
        None => String::new(),
    }
}
