use std::backtrace::Backtrace;
use std::io::Error as IoError;

use collection::operations::types::CollectionError;
use segment::common::file_operations::FileStorageError;
use tempfile::PersistError;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
#[error("{0}")]
pub enum StorageError {
    #[error("Wrong input: {description}")]
    BadInput { description: String },
    #[error("Not found: {description}")]
    NotFound { description: String },
    #[error("Service internal error: {description}")]
    ServiceError {
        description: String,
        backtrace: Option<String>,
    },
    #[error("Bad request: {description}")]
    BadRequest { description: String },
    #[error("Storage locked: {description}")]
    Locked { description: String },
}

impl StorageError {
    pub fn service_error(description: impl Into<String>) -> StorageError {
        StorageError::ServiceError {
            description: description.into(),
            backtrace: Some(Backtrace::force_capture().to_string()),
        }
    }

    pub fn bad_request(description: &str) -> StorageError {
        StorageError::BadRequest {
            description: description.to_string(),
        }
    }

    pub fn bad_input(description: &str) -> StorageError {
        StorageError::BadInput {
            description: description.to_string(),
        }
    }

    /// Used to override the `description` field of the resulting `StorageError`
    pub fn from_inconsistent_shard_failure(
        err: CollectionError,
        overriding_description: String,
    ) -> StorageError {
        match err {
            CollectionError::BadInput { .. } => StorageError::BadInput {
                description: overriding_description,
            },
            CollectionError::NotFound { .. } => StorageError::NotFound {
                description: overriding_description,
            },
            CollectionError::PointNotFound { .. } => StorageError::NotFound {
                description: overriding_description,
            },
            CollectionError::ServiceError { backtrace, .. } => StorageError::ServiceError {
                description: overriding_description,
                backtrace,
            },
            CollectionError::BadRequest { .. } => StorageError::BadRequest {
                description: overriding_description,
            },
            CollectionError::Cancelled { .. } => StorageError::ServiceError {
                description: format!("Operation cancelled: {overriding_description}"),
                backtrace: None,
            },
            CollectionError::InconsistentShardFailure { ref first_err, .. } => {
                StorageError::from_inconsistent_shard_failure(
                    *first_err.clone(),
                    overriding_description,
                )
            }
            CollectionError::BadShardSelection { .. } => StorageError::BadRequest {
                description: overriding_description,
            },
            CollectionError::ForwardProxyError { error, .. } => {
                Self::from_inconsistent_shard_failure(*error, overriding_description)
            }
        }
    }
}

impl From<CollectionError> for StorageError {
    fn from(err: CollectionError) -> Self {
        match err {
            CollectionError::BadInput { description } => StorageError::BadInput { description },
            CollectionError::NotFound { .. } => StorageError::NotFound {
                description: format!("{err}"),
            },
            CollectionError::PointNotFound { .. } => StorageError::NotFound {
                description: format!("{err}"),
            },
            CollectionError::ServiceError { error, backtrace } => StorageError::ServiceError {
                description: error,
                backtrace,
            },
            CollectionError::BadRequest { description } => StorageError::BadRequest { description },
            CollectionError::Cancelled { description } => StorageError::ServiceError {
                description: format!("Operation cancelled: {description}"),
                backtrace: None,
            },
            CollectionError::InconsistentShardFailure { ref first_err, .. } => {
                let full_description = format!("{}", &err);
                StorageError::from_inconsistent_shard_failure(*first_err.clone(), full_description)
            }
            CollectionError::BadShardSelection { description } => {
                StorageError::BadRequest { description }
            }
            CollectionError::ForwardProxyError { error, .. } => {
                let full_description = format!("{error}");
                StorageError::from_inconsistent_shard_failure(*error, full_description)
            }
        }
    }
}

impl From<IoError> for StorageError {
    fn from(err: IoError) -> Self {
        StorageError::service_error(format!("{err}"))
    }
}

impl From<FileStorageError> for StorageError {
    fn from(err: FileStorageError) -> Self {
        match err {
            FileStorageError::IoError { description } => StorageError::service_error(description),
            FileStorageError::UserAtomicIoError => {
                StorageError::service_error("Unknown atomic write error")
            }
            FileStorageError::GenericError { description } => {
                StorageError::service_error(description)
            }
        }
    }
}

impl<Guard> From<std::sync::PoisonError<Guard>> for StorageError {
    fn from(err: std::sync::PoisonError<Guard>) -> Self {
        StorageError::ServiceError {
            description: format!("Mutex lock poisoned: {err}"),
            backtrace: Some(Backtrace::force_capture().to_string()),
        }
    }
}

impl<T> From<std::sync::mpsc::SendError<T>> for StorageError {
    fn from(err: std::sync::mpsc::SendError<T>) -> Self {
        StorageError::ServiceError {
            description: format!("Channel closed: {err}"),
            backtrace: Some(Backtrace::force_capture().to_string()),
        }
    }
}

impl From<tokio::sync::oneshot::error::RecvError> for StorageError {
    fn from(err: tokio::sync::oneshot::error::RecvError) -> Self {
        StorageError::ServiceError {
            description: format!("Channel sender dropped: {err}"),
            backtrace: Some(Backtrace::force_capture().to_string()),
        }
    }
}

impl From<serde_cbor::Error> for StorageError {
    fn from(err: serde_cbor::Error) -> Self {
        StorageError::ServiceError {
            description: format!("cbor (de)serialization error: {err}"),
            backtrace: Some(Backtrace::force_capture().to_string()),
        }
    }
}

impl From<prost::EncodeError> for StorageError {
    fn from(err: prost::EncodeError) -> Self {
        StorageError::ServiceError {
            description: format!("prost encode error: {err}"),
            backtrace: Some(Backtrace::force_capture().to_string()),
        }
    }
}

impl From<prost::DecodeError> for StorageError {
    fn from(err: prost::DecodeError) -> Self {
        StorageError::ServiceError {
            description: format!("prost decode error: {err}"),
            backtrace: Some(Backtrace::force_capture().to_string()),
        }
    }
}

impl From<raft::Error> for StorageError {
    fn from(err: raft::Error) -> Self {
        StorageError::ServiceError {
            description: format!("Error in Raft consensus: {err}"),
            backtrace: Some(Backtrace::force_capture().to_string()),
        }
    }
}

impl<E: std::fmt::Display> From<atomicwrites::Error<E>> for StorageError {
    fn from(err: atomicwrites::Error<E>) -> Self {
        StorageError::ServiceError {
            description: format!("Failed to write file: {err}"),
            backtrace: Some(Backtrace::force_capture().to_string()),
        }
    }
}

impl From<tonic::transport::Error> for StorageError {
    fn from(err: tonic::transport::Error) -> Self {
        StorageError::ServiceError {
            description: format!("Tonic transport error: {err}"),
            backtrace: Some(Backtrace::force_capture().to_string()),
        }
    }
}

impl From<reqwest::Error> for StorageError {
    fn from(err: reqwest::Error) -> Self {
        StorageError::ServiceError {
            description: format!("Http request error: {err}"),
            backtrace: Some(Backtrace::force_capture().to_string()),
        }
    }
}

impl From<tokio::task::JoinError> for StorageError {
    fn from(err: tokio::task::JoinError) -> Self {
        StorageError::ServiceError {
            description: format!("Tokio task join error: {err}"),
            backtrace: Some(Backtrace::force_capture().to_string()),
        }
    }
}

impl From<PersistError> for StorageError {
    fn from(err: PersistError) -> Self {
        StorageError::ServiceError {
            description: format!("Persist error: {err}"),
            backtrace: Some(Backtrace::force_capture().to_string()),
        }
    }
}
