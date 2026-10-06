//! Reserved test values from https://docs.pulseshiga.io/testing.

use crate::config::Person;

pub struct Outcome {
    pub label: &'static str,
    pub evm: &'static str,
    pub solana: &'static str,
    pub account: &'static str,
}

pub const OUTCOMES: [Outcome; 5] = [
    Outcome {
        label: "Completed",
        evm: "0x0000000000000000000000000000000000000001",
        solana: "sandboxsuccess11111111111111111111111111111",
        account: "1111111111",
    },
    Outcome {
        label: "Completed after about 10 min",
        evm: "0x00000000000000000000000000000000001a66ed",
        solana: "sandboxs1owsuccess1111111111111111111111111",
        account: "7777777777",
    },
    Outcome {
        label: "Payment rejected",
        evm: "0x000000000000000000000000000000000000dead",
        solana: "sandboxpaymentrejected111111111111111111111",
        account: "2222222222",
    },
    Outcome {
        label: "Local payout fails",
        evm: "0x0000000000000000000000000000000000fa11ed",
        solana: "sandboxpayoutfai1ed111111111111111111111111",
        account: "5555555555",
    },
    Outcome {
        label: "Expires after about 30 s",
        evm: "0x00000000000000000000000000000000000faded",
        solana: "sandboxexpired11111111111111111111111111111",
        account: "9999999999",
    },
];

pub fn address(outcome: usize, network: &str) -> &'static str {
    let o = &OUTCOMES[outcome];
    if network == "SOLANA" { o.solana } else { o.evm }
}

pub fn identity() -> Person {
    Person {
        name: "Ada Obi".into(),
        email: "ada@example.com".into(),
        nin: "12345678901".into(),
        bvn: "22345678901".into(),
    }
}
