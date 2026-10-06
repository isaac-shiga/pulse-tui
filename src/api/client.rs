use std::time::Duration;

use reqwest::Method;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::{ApiError, Bank, CreateOrderRequest, Env, Order, Quote, QuoteRequest, VerifiedAccount};

/// Splits the `{status, data, meta}` envelope, or returns the API error.
fn envelope(mut body: Value) -> Result<(Value, Value), ApiError> {
    if body.get("status").and_then(Value::as_bool) == Some(true) {
        let data = body.get_mut("data").map(Value::take).unwrap_or_default();
        let meta = body.get_mut("meta").map(Value::take).unwrap_or_default();
        return Ok((data, meta));
    }
    let code = body
        .get("code")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let message = body
        .get("error")
        .and_then(Value::as_str)
        .unwrap_or("Pulse returned an error.");
    Err(ApiError::new(code, message))
}

pub(crate) fn parse<T: DeserializeOwned>(body: Value) -> Result<T, ApiError> {
    let (data, _) = envelope(body)?;
    serde_json::from_value(data)
        .map_err(|e| ApiError::new("bad_response", format!("Unexpected response: {e}")))
}

pub fn http() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent(concat!("pulse-tui/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("HTTP client")
}

#[derive(Clone)]
pub struct Client {
    http: reqwest::blocking::Client,
    base: &'static str,
    key: String,
}

impl Client {
    pub fn new(http: reqwest::blocking::Client, env: Env, key: String) -> Self {
        Self {
            http,
            base: env.base_url(),
            key,
        }
    }

    fn call(&self, method: Method, path: &str, body: Option<&Value>) -> Result<Value, ApiError> {
        let mut req = self
            .http
            .request(method, format!("{}{path}", self.base))
            .header("X-API-Key", &self.key);
        if let Some(body) = body {
            req = req.json(body);
        }
        let resp = req
            .send()
            .map_err(|e| ApiError::new("network", e.to_string()))?;
        let status = resp.status();
        resp.json::<Value>().map_err(|_| {
            ApiError::new(
                "bad_response",
                format!("HTTP {status} with a body that is not JSON."),
            )
        })
    }

    pub fn create_quote(&self, req: &QuoteRequest) -> Result<Quote, ApiError> {
        let body = serde_json::to_value(req).expect("quote request");
        parse(self.call(Method::POST, "/v1/quotes", Some(&body))?)
    }

    pub fn refresh_quote(&self, id: &str) -> Result<Quote, ApiError> {
        parse(self.call(Method::POST, &format!("/v1/quotes/{id}/refresh"), None)?)
    }

    pub fn create_order(&self, request: &CreateOrderRequest) -> Result<Order, ApiError> {
        let (path, body) = match request {
            CreateOrderRequest::Onramp(body) => ("/v1/onramp/orders", serde_json::to_value(body)),
            CreateOrderRequest::Offramp(body) => ("/v1/offramp/orders", serde_json::to_value(body)),
        };
        let body = body.expect("order request");
        parse(self.call(Method::POST, path, Some(&body))?)
    }

    pub fn order(&self, id: &str) -> Result<Order, ApiError> {
        parse(self.call(Method::GET, &format!("/v1/orders/{id}"), None)?)
    }

    pub fn orders(
        &self,
        kind: Option<&str>,
        cursor: Option<&str>,
    ) -> Result<(Vec<Order>, Option<String>), ApiError> {
        let mut path = "/v1/orders?per_page=50".to_string();
        if let Some(kind) = kind {
            path.push_str(&format!("&type={kind}"));
        }
        if let Some(cursor) = cursor {
            path.push_str(&format!("&cursor={cursor}"));
        }
        let (data, meta) = envelope(self.call(Method::GET, &path, None)?)?;
        let orders = serde_json::from_value(data)
            .map_err(|e| ApiError::new("bad_response", format!("Unexpected response: {e}")))?;
        let next = meta
            .get("next_cursor")
            .and_then(Value::as_str)
            .map(String::from);
        Ok((orders, next))
    }

    pub fn banks(&self) -> Result<Vec<Bank>, ApiError> {
        parse(self.call(Method::GET, "/v1/banks", None)?)
    }

    pub fn resolve(
        &self,
        bank_code: &str,
        account_number: &str,
    ) -> Result<VerifiedAccount, ApiError> {
        let body = serde_json::json!({ "bank_code": bank_code, "account_number": account_number });
        parse(self.call(Method::POST, "/v1/banks/resolve", Some(&body))?)
    }
}
