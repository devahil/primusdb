use axum::{
    extract::ws::{WebSocket, WebSocketUpgrade},
    extract::State,
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::broadcast;

use super::AppState;

/// Shared real-time event broadcast state.
///
/// Holds the [`tokio::sync::broadcast`] sender that fans events out to every
/// connected [`ws_handler`] WebSocket and [`sse_handler`] Server-Sent-Events
/// subscriber. A single `Arc<WsState>` is created in
/// [`crate::api::APIServer::with_network_config`] and stored on
/// [`crate::api::AppState`].
#[derive(Clone)]
pub struct WsState {
    /// Broadcast sender that fans events out to all active subscribers.
    pub broadcast_tx: broadcast::Sender<WsMessage>,
}

/// A single real-time event pushed over WebSocket and SSE connections.
///
/// # Fields
/// * `event_type` - Machine-readable event name (for example `record.created`,
///   `record.updated`, `record.deleted` or `table.truncated`).
/// * `data` - Arbitrary JSON payload associated with the event.
/// * `timestamp` - Unix epoch seconds at which the event was produced.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct WsMessage {
    pub event_type: String,
    pub data: serde_json::Value,
    pub timestamp: u64,
}

impl WsState {
    /// Create a new `WsState` with the given broadcast channel capacity.
    ///
    /// # Arguments
    /// * `capacity` - Number of events buffered for a lagging subscriber before
    ///   it starts dropping the oldest events.
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { broadcast_tx: tx }
    }

    /// Publish an event to every connected WebSocket and SSE subscriber.
    ///
    /// A message sent before any subscriber connects is discarded.
    pub fn broadcast(&self, msg: WsMessage) {
        let _ = self.broadcast_tx.send(msg);
    }
}

/// Axum WebSocket upgrade handler serving `GET /api/v1/ws`.
///
/// Accepts the upgrade and spawns two tasks per connection: one forwards
/// broadcast events from [`WsState`] to the client as text frames, and one
/// receives client messages (logged as subscription intents). The connection
/// closes when either task ends or the client sends a close frame.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let ws_state = state.ws_state.clone();
    ws.on_upgrade(move |socket| handle_socket(socket, ws_state))
}

/// Drive a single upgraded WebSocket connection until it closes.
async fn handle_socket(socket: WebSocket, state: Arc<WsState>) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.broadcast_tx.subscribe();

    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            let json = serde_json::to_string(&msg).unwrap_or_default();
            if sender
                .send(axum::extract::ws::Message::Text(json))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                axum::extract::ws::Message::Text(text) => {
                    if let Ok(sub) = serde_json::from_str::<serde_json::Value>(&text) {
                        tracing::info!("WS subscription: {:?}", sub);
                    }
                }
                axum::extract::ws::Message::Close(_) => break,
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }
}

/// Axum Server-Sent Events handler serving `GET /api/v1/sse`.
///
/// Subscribes to the same [`WsState`] broadcast channel as [`ws_handler`] and
/// streams each event as an `event:` / `data:` SSE frame, keeping the
/// connection alive with a ping every 30 seconds.
pub async fn sse_handler(
    State(state): State<Arc<AppState>>,
) -> axum::response::Sse<
    impl futures::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>,
> {
    let mut rx = state.ws_state.broadcast_tx.subscribe();

    let stream = async_stream::stream! {
        while let Ok(msg) = rx.recv().await {
            let data = serde_json::to_string(&msg).unwrap_or_default();
            yield Ok(axum::response::sse::Event::default()
                .event(&msg.event_type)
                .data(data));
        }
    };

    axum::response::Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::default()
            .interval(std::time::Duration::from_secs(30))
            .text("ping"),
    )
}
