use serde::{Deserialize, Serialize};

/// Identifies which authentication scheme a request uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthType {
    None,
    Basic,
    Bearer,
    ApiKey,
}

impl AuthType {
    pub const ALL: [AuthType; 4] = [AuthType::None, AuthType::Basic, AuthType::Bearer, AuthType::ApiKey];

    pub fn label(&self) -> &'static str {
        match self {
            AuthType::None => "No Auth",
            AuthType::Basic => "Basic",
            AuthType::Bearer => "Bearer Token",
            AuthType::ApiKey => "API Key",
        }
    }
}

/// Controls where an API key auth value is injected.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiKeyPlacement {
    Header,
    Query,
}

impl ApiKeyPlacement {
    pub const ALL: [ApiKeyPlacement; 2] = [ApiKeyPlacement::Header, ApiKeyPlacement::Query];

    pub fn label(&self) -> &'static str {
        match self {
            ApiKeyPlacement::Header => "Header",
            ApiKeyPlacement::Query => "Query Param",
        }
    }
}

/// A single user-entered request header/param/cookie pair.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Header {
    pub key: String,
    pub value: String,
    pub enabled: bool,
}

impl Header {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Header { key: key.into(), value: value.into(), enabled: true }
    }
}

/// Holds the fields for every supported auth type. Only the fields
/// relevant to `auth_type` are used when building a request.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthConfig {
    pub auth_type: AuthType,

    pub username: String,
    pub password: String,

    pub token: String,

    pub api_key_name: String,
    pub api_key_value: String,
    pub api_key_placement: ApiKeyPlacement,
}

impl Default for AuthConfig {
    fn default() -> Self {
        AuthConfig {
            auth_type: AuthType::None,
            username: String::new(),
            password: String::new(),
            token: String::new(),
            api_key_name: String::new(),
            api_key_value: String::new(),
            api_key_placement: ApiKeyPlacement::Header,
        }
    }
}

pub const METHODS: [&str; 7] = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

/// The full description of an HTTP call the user builds via the guided flow.
/// Auth-derived headers are intentionally not part of `headers`: they are
/// computed at send time so they can never silently collide with or be
/// duplicated alongside user-entered headers.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Request {
    pub method: String,
    pub url: String,
    /// query params merged onto the URL at send time
    pub params: Vec<Header>,
    pub headers: Vec<Header>,
    /// name/value pairs sent as a single Cookie header at send time
    pub cookies: Vec<Header>,
    pub auth: AuthConfig,
    pub body: String,
}

impl Default for Request {
    fn default() -> Self {
        Request {
            method: "GET".to_string(),
            url: String::new(),
            params: Vec::new(),
            headers: Vec::new(),
            cookies: Vec::new(),
            auth: AuthConfig::default(),
            body: String::new(),
        }
    }
}
