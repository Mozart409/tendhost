//! HTTP client for tendhost daemon

use reqwest::Client;
use tendhost_api::{requests::*, responses::*};

/// HTTP client for communicating with tendhost daemon
pub struct HttpClient {
    client: Client,
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
