//! WebSocket event types

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type")]
pub enum WsEvent {
    HostStateChanged {
        host: String,
        from: String,
        to: String,
    },
    UpdateProgress {
        host: String,
        package: String,
        progress: u8,
    },
    UpdateCompleted {
        host: String,
        result: String,
    },
    HostConnected {
        host: String,
    },
    HostDisconnected {
        host: String,
        reason: String,
    },
}
