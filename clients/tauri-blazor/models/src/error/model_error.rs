use crate::ErrorLocation;

use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum ModelError {
    #[error("Validation Error: {message} {location}")]
    Validation {
        message: String,
        location: ErrorLocation,
    },
}
