use serde::{Deserialize, Serialize};
use thiserror::Error;
use tonic::Status;

use crate::vault_handler::VaultHandlerError;

#[derive(Debug, Error, Serialize, Deserialize, Clone)]
pub enum VaultServiceError {
    #[error("{0}")]
    VaultHandlerError(#[from] VaultHandlerError),
}

static CUSTOM_ERROR: &str = "x-custom-tonic-error-vault_service_error";

impl TryFrom<Status> for VaultServiceError {
    type Error = ();

    fn try_from(status: Status) -> Result<Self, Self::Error> {
        match status.code() {
            tonic::Code::Internal => {
                if let Some(err) = status.metadata().get(CUSTOM_ERROR) {
                    Ok(serde_json::from_str(err.to_str().unwrap()).unwrap())
                } else {
                    Err(())
                }
            }
            _ => Err(()),
        }
    }
}

impl From<VaultServiceError> for Status {
    fn from(e: VaultServiceError) -> Self {
        let mut status = Status::internal(format!("internal error: {}", e));

        status.metadata_mut().insert(
            CUSTOM_ERROR,
            serde_json::to_string(&e)
                .unwrap_or("could not serialize: {e}".to_string())
                .parse()
                .unwrap_or(tonic::metadata::MetadataValue::from_static(
                    "unable to create metadata value",
                )),
        );
        status
    }
}
