use std::{error, fmt, sync::mpsc};
use async_std::{io, channel::SendError};

// Custom result type definition
pub type SessionResult<T> = Result<T, SessionError>;

// SessionError serves as the error type for session management
#[derive(Debug)]
pub enum SessionError {
    IoError(io::Error), // IoError wraps async I/O errors (for network & file operations)
    ChannelRecvError(mpsc::RecvError),
    // + Other error types?
    // Custom error serves as a catch-all
    Custom(String),
}

// Implement the StdError trait to ensure compatibility with 
// standard error handling mechanisms (e.g. "?")
impl error::Error for SessionError {
    // Implementing source() allows error-chaining
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            SessionError::IoError(err) => Some(err),
            SessionError::ChannelRecvError(err) => Some(err),
            // For custom errors where an underlying source is absent, return None.
            _ => None,
        }
    }
}

// Display trait to control how errors are presented as strings
impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionError::IoError(err) => write!(f, "I/O error: {}", err),
            SessionError::ChannelRecvError(err) => write!(f, "Channel receive error: {}", err),
            SessionError::Custom(msg) => write!(f, "Custom error: {}", msg),
        }
    }
}

// Implement From traits:
// Conversion from io::Error unifies error handling for I/O-related tasks
impl From<io::Error> for SessionError {
    fn from(err: io::Error) -> Self {
        SessionError::IoError(err)
    }
}

// Conversion from channel receiving errors
impl From<mpsc::RecvError> for SessionError {
    fn from(err: mpsc::RecvError) -> Self {
        SessionError::ChannelRecvError(err)
    }
}

// General-purpose conversion for boxed Error types
impl From<Box<dyn std::error::Error>> for SessionError {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        SessionError::Custom(err.to_string())
    }
}

// Handles boxed Error types that also implement Send
impl From<Box<dyn std::error::Error + Send>> for SessionError {
    fn from(err: Box<dyn std::error::Error + Send>) -> Self {
        SessionError::Custom(err.to_string())
    }
}

// Conversion from async_std::channel::SendError for String type
impl From<SendError<String>> for SessionError {
    fn from(err: SendError<String>) -> Self {
        SessionError::Custom(format!("Channel send error: {}", err))
    }
}

// Conversion from String, to convert simple error messages
impl From<String> for SessionError {
    fn from(err: String) -> Self {
        SessionError::Custom(err)
    }
}