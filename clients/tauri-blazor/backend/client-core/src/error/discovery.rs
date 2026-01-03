use models::{ErrorLocation, ModelError};

use std::error::Error as StdError;

use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum DiscoveryError {
    #[error("Network Query Error: {message} {location}")]
    NetworkQuery {
        message: String,
        location: ErrorLocation,
        #[source]
        source: Box<dyn StdError + Send + Sync>,
    },

    #[error("System Query Error: {message} {location}")]
    SystemQuery {
        message: String,
        location: ErrorLocation,
    },

    #[error("Validation Error: {message} {location}")]
    Validation {
        message: String,
        location: ErrorLocation,
    },
}

impl From<ModelError> for DiscoveryError {
    fn from(error: ModelError) -> Self {
        match error {
            ModelError::Validation { message, location } => {
                DiscoveryError::Validation { message, location }
            }
        }
    }
}
