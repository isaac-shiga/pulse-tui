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
    assert!(input.value.is_empty());
}
