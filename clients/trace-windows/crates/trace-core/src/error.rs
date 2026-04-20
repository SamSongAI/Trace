use thiserror::Error;

#[derive(Debug, Error)]
pub enum TraceError {
    #[error("invalid vault path: {0}")]
    InvalidVaultPath(String),

    #[error("invalid thread config: {0}")]
    InvalidThreadConfig(String),

    #[error("path escapes vault: {0}")]
    PathEscapesVault(String),

    #[error("invalid filename: {0}")]
    InvalidFilename(String),

    #[error("invalid target folder: {0}")]
    InvalidTargetFolderPath(String),

    #[error("unsupported date pattern token: {0}")]
    UnsupportedDatePatternToken(String),

    #[error("atomic write failed: {0}")]
    AtomicWriteFailed(String),

    #[error("image encoding failed: {0}")]
    ImageEncodingFailed(String),

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
    fn display_formats_path_escapes_vault() {
        let err = TraceError::PathEscapesVault("../etc/passwd".into());
        assert_eq!(err.to_string(), "path escapes vault: ../etc/passwd");
    }

    #[test]
    fn display_formats_invalid_filename() {
        let err = TraceError::InvalidFilename("...".into());
        assert_eq!(err.to_string(), "invalid filename: ...");
    }

    #[test]
    fn display_formats_invalid_target_folder_path() {
        let err = TraceError::InvalidTargetFolderPath("..".into());
        assert_eq!(err.to_string(), "invalid target folder: ..");
    }

    #[test]
    fn display_formats_unsupported_date_pattern_token() {
        let err = TraceError::UnsupportedDatePatternToken("Q".into());
        assert_eq!(err.to_string(), "unsupported date pattern token: Q");
    }

    #[test]
    fn display_formats_atomic_write_failed() {
        let err = TraceError::AtomicWriteFailed("rename aborted".into());
        assert_eq!(err.to_string(), "atomic write failed: rename aborted");
    }

    #[test]
    fn display_formats_image_encoding_failed() {
        let err = TraceError::ImageEncodingFailed("png encoder refused".into());
        assert_eq!(
            err.to_string(),
            "image encoding failed: png encoder refused"
        );
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
