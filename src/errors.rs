use std::{error, fmt};
use async_std::{io, channel::SendError};

// Custom result type definition
pub type SessionResult<T> = Result<T, SessionError>;

// SessionError serves as the error type for session management
#[derive(Debug)]
pub enum SessionError {
    IoError(io::Error), // IoError wraps async I/O errors (for network & file operations)
    ChannelSendError(String), 
    ChannelRecvError(async_std::channel::RecvError),
    ClientDisconnected,
    TimeoutError,
    // + Other error types?
    // Custom error serves as a catch-all
    Custom(String),
}

// Implement the StdError trait to ensure compatibility with 
// standard error handling mechanisms (e.g. "?")
impl error::Error for SessionError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            SessionError::IoError(err) => Some(err),
            SessionError::ChannelRecvError(err) => Some(err),
            // For custom errors where an underlying source is absent: return None
            _ => None,
        }
    }
}

// Display trait to control how errors are presented as strings
impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionError::IoError(err) => write!(f, "I/O error: {}", err),
            SessionError::ChannelSendError(err) => write!(f, "Channel send error: {}", err),
            SessionError::ChannelRecvError(err) => write!(f, "Channel receive error: {}", err),
            SessionError::ClientDisconnected => write!(f, "Client disconnected"), 
            SessionError::TimeoutError => write!(f, "Timeout"), 
            SessionError::Custom(msg) => write!(f, "Custom error: {}", msg),
        }
    }
}

// Implement From traits:
// Conversion from io::Error
impl From<io::Error> for SessionError {
    fn from(err: io::Error) -> Self {
        SessionError::IoError(err)
    }
}

// Conversion from channel receiving errors
impl From<async_std::channel::RecvError> for SessionError {
    fn from(err: async_std::channel::RecvError) -> Self {
        SessionError::ChannelRecvError(err)
    }
}

// General-purpose conversion for boxed Error types
impl From<Box<dyn std::error::Error + Send>> for SessionError {
    fn from(err: Box<dyn std::error::Error + Send>) -> Self {
        SessionError::Custom(err.to_string())
    }
}

// Conversion from async_std::channel::SendError for u8 type
impl From<SendError<Vec<u8>>> for SessionError {
    fn from(err: SendError<Vec<u8>>) -> Self {
        SessionError::Custom(format!("Channel send error: {}", err))
    }
}

// Conversion from String, to convert simple error messages
impl From<String> for SessionError {
    fn from(err: String) -> Self {
        SessionError::Custom(err)
    }
}

// Infallible is used for conversions that cannot fail, so just panic if this ever happens
impl From<std::convert::Infallible> for SessionError {
    fn from(_: std::convert::Infallible) -> Self {
        panic!("Infallible error should never be converted to SessionError")
    }
}

impl From<async_std::channel::SendError<crate::session::SessionEvent>> for SessionError {
    fn from(err: async_std::channel::SendError<crate::session::SessionEvent>) -> Self {
        SessionError::ChannelSendError(format!("Channel send error: {}", err))
    }
}