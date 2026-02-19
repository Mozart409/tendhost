//! HTTP client for tendhost daemon

use reqwest::Client;

/// HTTP client for communicating with tendhost daemon
pub struct HttpClient {
    #[allow(dead_code)]
    client: Client,
    #[allow(dead_code)]
    base_url: String,
}

impl HttpClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into(),
        }
    }
}
