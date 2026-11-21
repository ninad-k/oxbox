use thiserror::Error;

/// Every fallible function in `oxbox-core` returns this error.
///
/// The Tauri layer turns it into `{ code, message }` for the UI, so keep
/// messages short, human-readable and free of secrets or full file contents.
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("This tool is not available on {0}")]
    Unsupported(&'static str),

    #[error("Cancelled")]
    Cancelled,
}

impl CoreError {
    /// Stable machine-readable code the UI can switch on.
    pub fn code(&self) -> &'static str {
        match self {
            CoreError::Io(e) if e.kind() == std::io::ErrorKind::NotFound => "not_found",
            CoreError::Io(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                "permission_denied"
            }
            CoreError::Io(_) => "io",
            CoreError::InvalidInput(_) => "invalid_input",
            CoreError::Unsupported(_) => "unsupported",
            CoreError::Cancelled => "cancelled",
        }
    }
}

pub type CoreResult<T> = Result<T, CoreError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_not_found_maps_to_stable_code() {
        let err: CoreError = std::io::Error::from(std::io::ErrorKind::NotFound).into();
        assert_eq!(err.code(), "not_found");
    }

    #[test]
    fn invalid_input_has_readable_message() {
        let err = CoreError::InvalidInput("empty text".into());
        assert_eq!(err.to_string(), "Invalid input: empty text");
        assert_eq!(err.code(), "invalid_input");
    }
}
