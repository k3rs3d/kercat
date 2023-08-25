use std::fmt;
use std::error::Error as StdError;
use async_std::io;
use std::sync::mpsc;

// SessionError is the custom error type 
#[derive(Debug)]
pub enum SessionError {
    // IoError wraps async I/O errors (for network & file operations)
    IoError(io::Error),
    // ChannelRecvError, of course, handles errors when receiving from channels
    ChannelRecvError(mpsc::RecvError),
    // + Other error types?
    // Custom error serves as a catch-all
    Custom(String),
}

// Implement the StdError trait to ensure compatibility with 
// standard error handling mechanisms (e.g. "?")
impl StdError for SessionError {
    // Implement source to return underlying error type
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            SessionError::IoError(err) => Some(err),
            SessionError::ChannelRecvError(err) => Some(err),
            // For custom/nonstandard errors, return None:
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
// Conversion from io::Error
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

// Conversion from String, to convert simple error messages
impl From<String> for SessionError {
    fn from(err: String) -> Self {
        SessionError::Custom(err)
    }
}
