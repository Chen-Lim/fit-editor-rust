use std::fmt;

#[derive(Debug)]
pub enum CliError {
    /// Not a FIT file
    NotFit(String),
    /// File too short or truncated
    Truncated { expected: usize, actual: usize },
    /// CRC mismatch
    CrcMismatch {
        stored: u16,
        calculated: u16,
        which: &'static str,
    },
    /// Unknown message type
    #[allow(dead_code)]
    UnknownMessage(String),
    /// Invalid field path
    InvalidFieldPath(String),
    /// IO error
    Io(std::io::Error),
    /// JSON error
    Json(serde_json::Error),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFit(path) => write!(f, "{path}: not a FIT file"),
            Self::Truncated { expected, actual } => {
                write!(
                    f,
                    "file truncated: expected {expected} bytes, got {actual}"
                )
            }
            Self::CrcMismatch {
                stored,
                calculated,
                which,
            } => {
                write!(
                    f,
                    "{which} CRC mismatch: stored 0x{stored:04X}, calculated 0x{calculated:04X}"
                )
            }
            Self::UnknownMessage(name) => write!(f, "unknown message type: {name}"),
            Self::InvalidFieldPath(path) => write!(f, "invalid field path: {path}"),
            Self::Io(e) => write!(f, "IO error: {e}"),
            Self::Json(e) => write!(f, "JSON error: {e}"),
        }
    }
}

impl std::error::Error for CliError {}

impl From<std::io::Error> for CliError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_json::Error> for CliError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

impl From<csv::Error> for CliError {
    fn from(e: csv::Error) -> Self {
        Self::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

impl From<fit::FitError> for CliError {
    fn from(e: fit::FitError) -> Self {
        match e {
            fit::FitError::HeaderCrcMismatch { stored, calculated } => Self::CrcMismatch {
                stored,
                calculated,
                which: "Header",
            },
            fit::FitError::FileCrcMismatch { stored, calculated } => Self::CrcMismatch {
                stored,
                calculated,
                which: "File",
            },
            fit::FitError::TooShort { expected, actual } => Self::Truncated { expected, actual },
            other => Self::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, other)),
        }
    }
}
