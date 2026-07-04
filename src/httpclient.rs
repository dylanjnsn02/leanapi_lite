use std::collections::BTreeMap;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use anyhow::Result;
use reqwest::Url;

use crate::history::ResponseSnapshot;
use crate::model::{ApiKeyPlacement, AuthConfig, AuthType, Header, Request};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

static CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();

fn client() -> &'static reqwest::blocking::Client {
    CLIENT.get_or_init(|| {
        reqwest::blocking::Client::builder()
            .timeout(DEFAULT_TIMEOUT)
            .build()
            .expect("failed to build http client")
    })
}

/// Computes the header(s) implied by an auth config. Never mutates or reads
/// req.headers -- callers apply these first so a user-entered header with
/// the same name can override them.
pub fn derived_headers(auth: &AuthConfig) -> Vec<Header> {
    match auth.auth_type {
        AuthType::Basic => Vec::new(), // applied directly via .basic_auth() in build_request
        AuthType::Bearer => {
            if auth.token.is_empty() {
                Vec::new()
            } else {
                vec![Header::new("Authorization", format!("Bearer {}", auth.token))]
            }
        }
        AuthType::ApiKey => {
            if auth.api_key_placement == ApiKeyPlacement::Header && !auth.api_key_name.is_empty() {
                vec![Header::new(auth.api_key_name.clone(), auth.api_key_value.clone())]
            } else {
                Vec::new()
            }
        }
        AuthType::None => Vec::new(),
    }
}

/// Turns a model::Request into a ready-to-send reqwest::blocking::Request,
/// applying auth (as headers/query/basic-auth) before user headers so a user
/// header of the same name always wins (last write wins via .header()).
pub fn build_request(
    http_client: &reqwest::blocking::Client,
    req: &Request,
) -> Result<reqwest::blocking::Request> {
    let mut url = Url::parse(&req.url)?;

    let mut query: BTreeMap<String, String> = url.query_pairs().into_owned().collect();

    if req.auth.auth_type == AuthType::ApiKey
        && req.auth.api_key_placement == ApiKeyPlacement::Query
        && !req.auth.api_key_name.is_empty()
    {
        query.insert(req.auth.api_key_name.clone(), req.auth.api_key_value.clone());
    }

    for p in &req.params {
        if !p.enabled || p.key.is_empty() {
            continue;
        }
        query.insert(p.key.clone(), p.value.clone());
    }

    if query.is_empty() {
        url.set_query(None);
    } else {
        url.query_pairs_mut().clear().extend_pairs(query.iter());
    }

    let method = reqwest::Method::from_bytes(req.method.as_bytes()).unwrap_or(reqwest::Method::GET);

    let mut builder = http_client.request(method, url);

    if req.auth.auth_type == AuthType::Basic {
        builder = builder.basic_auth(&req.auth.username, Some(&req.auth.password));
    }

    for h in derived_headers(&req.auth) {
        builder = builder.header(h.key, h.value);
    }

    let cookie_header = req
        .cookies
        .iter()
        .filter(|c| c.enabled && !c.key.is_empty())
        .map(|c| format!("{}={}", c.key, c.value))
        .collect::<Vec<_>>()
        .join("; ");
    if !cookie_header.is_empty() {
        builder = builder.header(reqwest::header::COOKIE, cookie_header);
    }

    let mut has_content_type = false;
    for h in &req.headers {
        if !h.enabled || h.key.is_empty() {
            continue;
        }
        builder = builder.header(&h.key, &h.value);
        if h.key.eq_ignore_ascii_case("content-type") {
            has_content_type = true;
        }
    }

    if !req.body.is_empty() {
        if !has_content_type {
            builder = builder.header(reqwest::header::CONTENT_TYPE, "application/json");
        }
        builder = builder.body(req.body.clone());
    }

    Ok(builder.build()?)
}

/// The result of attempting to send a request: always carries how long it
/// took, even on failure, so callers can log a history entry either way.
pub struct SendOutcome {
    pub duration_ms: i64,
    pub result: Result<ResponseSnapshot, String>,
}

/// Builds and executes req, never panicking -- errors at any stage (bad URL,
/// connection failure, body read failure) are captured as a SendOutcome
/// error rather than propagated, mirroring the Go UI's ResponseMsg.
pub fn send_request(req: &Request) -> SendOutcome {
    let start = Instant::now();
    let http_client = client();

    let http_req = match build_request(http_client, req) {
        Ok(r) => r,
        Err(e) => {
            return SendOutcome { duration_ms: start.elapsed().as_millis() as i64, result: Err(e.to_string()) };
        }
    };

    let resp = match http_client.execute(http_req) {
        Ok(r) => r,
        Err(e) => {
            return SendOutcome { duration_ms: start.elapsed().as_millis() as i64, result: Err(e.to_string()) };
        }
    };

    let status_code = resp.status().as_u16() as i32;
    let status = format!(
        "{} {}",
        resp.status().as_u16(),
        resp.status().canonical_reason().unwrap_or("")
    )
    .trim_end()
    .to_string();

    let mut headers: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (k, v) in resp.headers().iter() {
        headers
            .entry(k.as_str().to_string())
            .or_default()
            .push(v.to_str().unwrap_or("").to_string());
    }

    let body_bytes = match resp.bytes() {
        Ok(b) => b,
        Err(e) => {
            return SendOutcome { duration_ms: start.elapsed().as_millis() as i64, result: Err(e.to_string()) };
        }
    };

    let duration_ms = start.elapsed().as_millis() as i64;
    let size = body_bytes.len() as i64;
    let body = String::from_utf8_lossy(&body_bytes).to_string();

    SendOutcome {
        duration_ms,
        result: Ok(ResponseSnapshot { status_code, status, headers, body, size }),
    }
}
