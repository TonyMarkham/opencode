pub mod discovery;
pub mod spawn;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error(transparent)]
    Discovery(#[from] discovery::DiscoveryError),

    #[error(transparent)]
    Spawn(#[from] spawn::SpawnError),
}
