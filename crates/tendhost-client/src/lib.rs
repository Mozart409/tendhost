//! tendhost-client: HTTP and WebSocket client library
//!
//! Provides both HTTP and WebSocket clients for communicating with the tendhost daemon.
//!
//! # Examples
//!
//! ## HTTP Client
//!
//! ```no_run
//! use tendhost_client::HttpClient;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = HttpClient::new("http://localhost:8080")?;
//!
//! // Get health
//! let health = client.health().await?;
//! println!("Status: {}", health.status);
//!
//! // List hosts with filters
//! let hosts = client.list_hosts()
//!     .page(1)
//!     .per_page(50)
//!     .tag("production")
//!     .send()
//!     .await?;
//!
//! // Get single host
//! let host = client.get_host("debian-vm").await?;
//!
//! // Trigger update
//! client.update_host_packages("debian-vm", false).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## WebSocket Client
//!
//! ```no_run
//! use tendhost_client::WsClient;
//! use tendhost_api::events::WsEvent;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut client = WsClient::connect("ws://localhost:8080/ws/events").await?;
//!
//! while let Some(event) = client.recv().await {
//!     match event {
//!         WsEvent::HostStateChanged { host, from, to } => {
//!             println!("{host}: {from} -> {to}");
//!         }
//!         WsEvent::UpdateProgress { host, package, progress } => {
//!             println!("{host}: {package} {progress}%");
//!         }
//!         _ => {}
//!     }
//! }
//! # Ok(())
//! # }
//! ```

pub mod error;
pub mod http;
pub mod ws;

pub use error::{ClientError, Result};
pub use http::{HttpClient, ListHostsBuilder};
pub use ws::WsClient;
