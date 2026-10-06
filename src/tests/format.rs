use crate::ui::format::{group, mask, money};

#[test]
fn groups_thousands_and_keeps_decimals() {
    assert_eq!(group("150000"), "150,000");
    assert_eq!(group("1538.46"), "1,538.46");
    assert_eq!(group("97.499512"), "97.499512");
    assert_eq!(money("NGN", "100000000"), "₦100,000,000");
    assert_eq!(money("USDT", "102.432779"), "102.432779 USDT");
}

#[test]
fn mask_handles_multibyte_characters() {
    assert_eq!(mask("clé-secrète"), "••••rète");
}
