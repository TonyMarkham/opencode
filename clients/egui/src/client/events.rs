use serde::Deserialize;
use crate::error::events::EventsError;
    #[error("http error: {0}")]
    Http(String),

#[derive(Debug, Deserialize)]
pub struct GlobalEvent<T = serde_json::Value> {
    pub directory: String,
    #[serde(bound = "T: Deserialize<'de>")]
    pub payload: T,
}

/// Placeholder; wire up with an SSE client crate (eventsource-stream or reqwest_eventsource)
/// returning a Stream of GlobalEvent. Keeping signature minimal for now.
pub struct EventStream; // TODO: Replace with actual Stream<Item = Result<GlobalEvent, EventsError>>

pub async fn subscribe_global(_base_url: &str) -> Result<EventStream, EventsError> {
    // TODO: implement with SSE client
    Err(EventsError::Http("not implemented".into()))
}
