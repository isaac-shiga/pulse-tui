use crate::api::*;
use serde_json::json;

#[test]
fn error_envelope_keeps_code_and_message() {
    let err = parse::<Quote>(
        json!({ "status": false, "error": "Quote expired", "code": "quote_expired" }),
    )
    .unwrap_err();
    assert_eq!(err.code, "quote_expired");
    assert_eq!(err.message, "Quote expired");
}

#[test]
fn order_without_funding_account_parses() {
    let order: Order = parse(json!({ "status": true, "data": {
        "id": "o1", "reference": "r1", "type": "onramp", "status": "awaiting_payment",
        "source": { "currency": "NGN", "amount": "150000" },
        "destination": { "currency": "USDT", "network": "BASE", "amount": "97.5" }
    }}))
    .unwrap();
    assert_eq!(order.status, Status::AwaitingPayment);
    assert!(order.funding_account.is_none());
}

#[test]
fn offramp_request_nests_the_bank_account() {
    let request = OfframpRequest {
        reference: "ref-1".into(),
        quote_id: "quote-1".into(),
        beneficiary: BeneficiaryRequest {
            name: "Ada Obi".into(),
            email: "ada@example.com".into(),
            nin: "12345678901".into(),
            bvn: "22345678901".into(),
            bank_account: BankAccountRequest {
                bank_code: "044".into(),
                account_number: "1111111111".into(),
            },
        },
    };

    let body = serde_json::to_value(request).unwrap();

    assert_eq!(body["beneficiary"]["bank_account"]["bank_code"], "044");
    assert!(body.get("bank_account").is_none());
}
