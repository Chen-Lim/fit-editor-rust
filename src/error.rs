use std::fmt;
use std::path::Path;

/// Default maximum file size: 256 MiB.
pub const DEFAULT_MAX_FILE_SIZE: u64 = 256 * 1024 * 1024;

/// Read a file into bytes, rejecting files larger than `max_bytes`.
pub fn read_bounded(path: &Path, max_bytes: u64) -> Result<Vec<u8>, CliError> {
    let meta = std::fs::metadata(path)?;
    if meta.len() > max_bytes {
        return Err(CliError::FileTooLarge {
            path: path.display().to_string(),
            size: meta.len(),
            max: max_bytes,
        });
    }
    Ok(std::fs::read(path)?)
}

/// Read a file to string, rejecting files larger than `max_bytes`.
pub fn read_to_string_bounded(path: &Path, max_bytes: u64) -> Result<String, CliError> {
    let meta = std::fs::metadata(path)?;
    if meta.len() > max_bytes {
        return Err(CliError::FileTooLarge {
            path: path.display().to_string(),
            size: meta.len(),
            max: max_bytes,
        });
    }
    Ok(std::fs::read_to_string(path)?)
}

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
    /// Invalid field path
    InvalidFieldPath(String),
    /// Bad usage / invalid arguments
    BadUsage(String),
    /// Batch had partial failures
    BatchPartialFailure { succeeded: usize, failed: usize },
    /// Decoder reported errors
    DecodeErrors(usize),
    /// File exceeds size limit
    FileTooLarge { path: String, size: u64, max: u64 },
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
                write!(f, "file truncated: expected {expected} bytes, got {actual}")
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
            Self::InvalidFieldPath(path) => write!(f, "invalid field path: {path}"),
            Self::BadUsage(msg) => write!(f, "bad usage: {msg}"),
            Self::BatchPartialFailure { succeeded, failed } => {
                write!(f, "{failed} of {} files failed", succeeded + failed)
            }
            Self::DecodeErrors(n) => {
                write!(f, "decoder reported {n} error(s)")
            }
            Self::FileTooLarge { path, size, max } => {
                write!(
                    f,
                    "{path}: file too large ({size} bytes, limit {max} bytes)"
                )
            }
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
