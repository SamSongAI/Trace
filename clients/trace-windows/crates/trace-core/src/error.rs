use thiserror::Error;

#[derive(Debug, Error)]
pub enum TraceError {
    #[error("invalid vault path: {0}")]
    InvalidVaultPath(String),

    #[error("invalid thread config: {0}")]
    InvalidThreadConfig(String),

    #[error("serialization failed: {0}")]
    SerializationFailed(#[from] serde_json::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_formats_invalid_vault_path() {
        let err = TraceError::InvalidVaultPath("/missing".into());
        assert_eq!(err.to_string(), "invalid vault path: /missing");
    }

    #[test]
    fn display_formats_invalid_thread_config() {
        let err = TraceError::InvalidThreadConfig("duplicate name".into());
        assert_eq!(err.to_string(), "invalid thread config: duplicate name");
    }

    #[test]
    fn serde_json_error_converts_into_serialization_failed() {
        let json_err = serde_json::from_str::<i32>("not json").unwrap_err();
        let err: TraceError = json_err.into();
        assert!(matches!(err, TraceError::SerializationFailed(_)));
    }

    #[test]
    fn io_error_converts_into_io_variant() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let err: TraceError = io_err.into();
        assert!(matches!(err, TraceError::Io(_)));
    }
}
