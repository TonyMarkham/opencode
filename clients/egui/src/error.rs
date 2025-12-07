pub mod api;
pub mod events;
pub mod discovery;
pub mod spawn;

// Optional prelude for convenient imports
pub mod prelude {
    pub use super::api::ApiError;
    pub use super::events::EventsError;
    pub use super::discovery::DiscoveryError;
    pub use super::spawn::SpawnError;
}
