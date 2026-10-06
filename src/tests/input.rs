use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{Input, Kind};

#[test]
fn secret_display_handles_multibyte_characters() {
    let mut input = Input::new(Kind::Text).secret();
    input.set("clé-secrète");

    assert_eq!(input.display(), "••••••••rète");
}

#[test]
fn delete_clears_the_field() {
    let mut input = Input::new(Kind::Text);
    input.set("value");

    let changed = input.handle(&KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));

    assert!(changed);
    assert!(input.value().is_empty());
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn arrow_keys_let_the_user_fix_a_typo_mid_word() {
    let mut input = Input::new(Kind::Text);
    input.set("Ada Ob");
    input.handle(&key(KeyCode::Left));
    input.handle(&key(KeyCode::Left));
    input.handle(&key(KeyCode::Char('x')));

    assert_eq!(input.value(), "Ada xOb");
}

#[test]
fn digit_fields_refuse_letters_and_extra_digits() {
    let mut input = Input::new(Kind::Digits(3));
    for c in ['1', 'a', '2', '3', '4'] {
        input.handle(&key(KeyCode::Char(c)));
    }

    assert_eq!(input.value(), "123");
}

#[test]
fn a_secret_ignores_the_arrow_keys() {
    let mut input = Input::new(Kind::Text).secret();
    input.set("sk_test_1");
    input.handle(&key(KeyCode::Left));
    input.handle(&key(KeyCode::Char('2')));

    assert_eq!(input.value(), "sk_test_12");
}
