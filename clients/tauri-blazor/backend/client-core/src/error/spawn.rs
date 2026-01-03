use models::ErrorLocation;
use models::ModelError;

use serde::de::StdError;
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum SpawnError {
    #[error("Spawn Error: {message} {location}")]
    Spawn {
        message: String,
        location: ErrorLocation,
        #[source]
        source: Box<dyn StdError + Send + Sync>,
    },

    #[error("Parse Error: {message} {location}")]
    Parse {
        message: String,
        location: ErrorLocation,
    },

    #[error("Timeout Error: {message} {location}")]
    Timeout {
        message: String,
        location: ErrorLocation,
    },

    #[error("Validation Error: {message} {location}")]
    Validation {
        message: String,
        location: ErrorLocation,
    },
}

impl From<ModelError> for SpawnError {
    fn from(error: ModelError) -> Self {
        match error {
            ModelError::Validation { message, location } => {
                SpawnError::Validation { message, location }
            }
        }
    }
}
