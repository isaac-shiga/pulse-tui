use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

fn seconds_until(timestamp: &str) -> Option<i64> {
    let expiry = DateTime::parse_from_rfc3339(timestamp).ok()?;
    Some((expiry.with_timezone(&Utc) - Utc::now()).num_seconds())
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Env {
    #[default]
    Test,
    Live,
}

impl Env {
    pub fn base_url(self) -> &'static str {
        match self {
            Env::Test => "https://engine-api.pilot.pulseshiga.io",
            Env::Live => "https://engine-api.pulseshiga.io",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Env::Test => "Test",
            Env::Live => "Live",
        }
    }

    pub fn toggle(self) -> Self {
        match self {
            Env::Test => Env::Live,
            Env::Live => Env::Test,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

impl ApiError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    /// One sentence the user can act on. Known codes get their own sentence.
    /// Any other code gets `fallback`, because a raw server message can carry
    /// internal details from a bank or provider.
    pub fn friendly_or(&self, fallback: &str) -> String {
        let text = match self.code.as_str() {
            "invalid_amount" => {
                "This amount is outside the limits. Buy ₦15,000 to ₦100,000,000. \
                 Sell at least 10 USDT or USDC, up to a ₦100,000,000 payout."
            }
            "quote_expired" => "The quote expired. A new price is on the way.",
            "quote_changed" => {
                "The price changed while the order was being created. Check the new price."
            }
            "invalid_destination_address" => "This address is not valid for the network.",
            "account_verification_failed" => "The bank could not confirm this account.",
            "unauthorized" => "The API key is not valid for this environment. Check Settings.",
            "rate_limited" => "Too many requests. Wait a minute, then try again.",
            "service_unavailable" => "Pulse is not available right now. Try again soon.",
            "network" => "Cannot reach Pulse. Check your connection, then try again.",
            _ => fallback,
        };
        text.to_string()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct QuoteRequest {
    pub source_currency: String,
    pub destination_currency: String,
    pub network: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_amount: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Quote {
    pub id: String,
    pub rate: String,
    pub source_amount: String,
    pub destination_amount: String,
    pub expires_at: String,
}

impl Quote {
    pub fn seconds_until_expiry(&self) -> Option<i64> {
        seconds_until(&self.expires_at)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PartyRequest {
    pub name: String,
    pub email: String,
    pub nin: String,
    pub bvn: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DestinationRequest {
    pub address: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BankAccountRequest {
    pub bank_code: String,
    pub account_number: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BeneficiaryRequest {
    pub name: String,
    pub email: String,
    pub nin: String,
    pub bvn: String,
    pub bank_account: BankAccountRequest,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OnrampRequest {
    pub reference: String,
    pub quote_id: String,
    pub payer: PartyRequest,
    pub destination: DestinationRequest,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OfframpRequest {
    pub reference: String,
    pub quote_id: String,
    pub beneficiary: BeneficiaryRequest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CreateOrderRequest {
    Onramp(OnrampRequest),
    Offramp(OfframpRequest),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    AwaitingPayment,
    Processing,
    Completed,
    Expired,
    Failed,
    #[serde(other)]
    Unknown,
}

impl Status {
    pub fn is_terminal(self) -> bool {
        matches!(self, Status::Completed | Status::Expired | Status::Failed)
    }

    pub fn label(self) -> &'static str {
        match self {
            Status::AwaitingPayment => "Awaiting payment",
            Status::Processing => "Processing",
            Status::Completed => "Completed",
            Status::Expired => "Expired",
            Status::Failed => "Failed",
            Status::Unknown => "Unknown",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct Money {
    #[serde(default)]
    pub currency: String,
    #[serde(default)]
    pub amount: String,
    #[serde(default)]
    pub network: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct FundingAccount {
    pub account_name: Option<String>,
    pub account_number: Option<String>,
    pub bank_code: Option<String>,
    pub bank_name: Option<String>,
    pub amount: Option<String>,
    pub deposit_address: Option<String>,
    pub network: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct Party {
    pub name: Option<String>,
    pub email: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Order {
    pub id: String,
    #[serde(default)]
    pub reference: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub status: Status,
    #[serde(default)]
    pub rate: Option<String>,
    #[serde(default)]
    pub source: Money,
    #[serde(default)]
    pub destination: Money,
    #[serde(default)]
    pub funding_account: Option<FundingAccount>,
    #[serde(default)]
    pub party: Option<Party>,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}

impl Order {
    pub fn is_onramp(&self) -> bool {
        self.kind == "onramp"
    }

    pub fn seconds_until_expiry(&self) -> Option<i64> {
        self.expires_at.as_deref().and_then(seconds_until)
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Bank {
    pub name: String,
    pub code: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct VerifiedAccount {
    pub account_number: String,
    pub account_name: String,
    pub bank_code: String,
    #[serde(default)]
    pub bank_name: String,
}
