//! WebSocket client for tendhost daemon

use std::time::Duration;

use futures::StreamExt;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;

use tendhost_api::events::WsEvent;

use crate::error::{ClientError, Result};

/// WebSocket client for receiving live events from tendhost daemon
#[derive(Debug)]
pub struct WsClient {
    #[allow(dead_code)]
    url: Url,
    receiver: mpsc::Receiver<WsEvent>,
    _task_handle: tokio::task::JoinHandle<()>,
}

impl WsClient {
    /// Connect to the WebSocket endpoint
    ///
    /// Automatically reconnects on connection loss with exponential backoff.
    ///
    /// # Errors
    /// Returns an error if the URL is invalid.
    ///
    /// # Example
    /// ```no_run
    /// use tendhost_client::WsClient;
    /// use tendhost_api::events::WsEvent;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut client = WsClient::connect("ws://localhost:8080/ws/events").await?;
    ///
    /// while let Some(event) = client.recv().await {
    ///     match event {
    ///         WsEvent::HostStateChanged { host, from, to } => {
    ///             println!("{host}: {from} -> {to}");
    ///         }
    ///         _ => {}
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[allow(clippy::unused_async)]
    pub async fn connect(url: impl AsRef<str>) -> Result<Self> {
        let url = Url::parse(url.as_ref())?;
        let (tx, rx) = mpsc::channel(100);

        let task_url = url.clone();
        let task_handle = tokio::spawn(async move {
            Self::connection_loop(task_url, tx).await;
        });

        Ok(Self {
            url,
            receiver: rx,
            _task_handle: task_handle,
        })
    }

    /// Receive the next event from the stream
    ///
    /// Returns `None` when the connection is closed and cannot be reconnected.
    pub async fn recv(&mut self) -> Option<WsEvent> {
        self.receiver.recv().await
    }

    /// Connection loop with auto-reconnection
    async fn connection_loop(url: Url, tx: mpsc::Sender<WsEvent>) {
        let mut backoff = Duration::from_secs(1);
        let max_backoff = Duration::from_secs(60);

        loop {
            match Self::connect_and_receive(&url, &tx).await {
                Ok(()) => {
                    // Connection closed gracefully
                    tracing::info!("WebSocket connection closed");
                    break;
                }
                Err(e) => {
                    tracing::warn!("WebSocket error: {}, reconnecting in {:?}", e, backoff);
                    sleep(backoff).await;

                    // Exponential backoff
                    backoff = (backoff * 2).min(max_backoff);
                }
            }
        }
    }

    /// Connect and receive messages
    async fn connect_and_receive(url: &Url, tx: &mpsc::Sender<WsEvent>) -> Result<()> {
        let (ws_stream, _) = connect_async(url.as_str())
            .await
            .map_err(|e| ClientError::WebSocket(e.to_string()))?;

        tracing::info!("WebSocket connected to {}", url);

        let (_write, mut read) = ws_stream.split();

        while let Some(msg) = read.next().await {
            let msg = msg.map_err(|e| ClientError::WebSocket(e.to_string()))?;

            match msg {
                Message::Text(text) => {
                    match serde_json::from_str::<WsEvent>(&text) {
                        Ok(event) => {
                            if tx.send(event).await.is_err() {
                                // Receiver dropped, exit
                                return Ok(());
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to parse event: {}", e);
                        }
                    }
                }
                Message::Close(_) => {
                    return Err(ClientError::ConnectionClosed(
                        "Server closed connection".into(),
                    ));
                }
                Message::Ping(_) | Message::Pong(_) | Message::Binary(_) | Message::Frame(_) => {
                    // Ping/pong handled automatically by tungstenite
                    // Ignore binary frames and raw frames
                }
            }
        }

        Err(ClientError::ConnectionClosed("Stream ended".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_parsing() {
        let url = Url::parse("ws://localhost:8080/ws/events");
        assert!(url.is_ok());
    }

    #[test]
    fn test_https_url() {
        let url = Url::parse("wss://example.com/ws/events");
        assert!(url.is_ok());
    }

    #[test]
    fn test_invalid_url() {
        let url = Url::parse("not a url");
        assert!(url.is_err());
    }
}
