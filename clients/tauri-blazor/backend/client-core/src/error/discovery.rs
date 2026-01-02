use common::ErrorLocation;

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
}
