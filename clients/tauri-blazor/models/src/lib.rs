//! Domain models for OpenCode.
//!
//! This crate contains pure data structures representing the core
//! concepts in our application. Models have no business logic - they're
//! just data that can be passed between layers.
//!
//! ## Architecture
//!
//! - **models** (this crate): Pure data structures
//! - **client-core**: Business logic operating on models
//! - **opencode**: Application wiring everything together
//!
//! This layered architecture keeps concerns separated and makes testing easier.

pub mod error;

mod server_info;
#[cfg(test)]
mod tests;

pub use error::error_location::ErrorLocation;
pub use error::model_error::ModelError;
pub use server_info::builder::ServerInfoBuilder;
pub use server_info::info::ServerInfo;
