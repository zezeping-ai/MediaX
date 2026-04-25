use std::fmt::{Display, Formatter};

use serde::Serialize;

#[derive(Debug)]
pub enum MediaError {
    InvalidInput(String),
    Internal(String),
    StatePoisoned(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct MediaCommandError {
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaErrorCode {
    InvalidInput,
    InternalError,
    DecodeFailed,
    StreamStartFailed,
    StatePoisoned,
}

impl MediaErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidInput => "INVALID_INPUT",
            Self::InternalError => "INTERNAL_ERROR",
            Self::DecodeFailed => "DECODE_FAILED",
            Self::StreamStartFailed => "STREAM_START_FAILED",
            Self::StatePoisoned => "STATE_POISONED",
        }
    }
}

impl MediaError {
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::InvalidInput(message.into())
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }

    pub fn state_poisoned(message: impl Into<String>) -> Self {
        Self::StatePoisoned(message.into())
    }

    pub fn state_poisoned_lock(state_name: &str) -> Self {
        Self::state_poisoned(format!("{state_name} lock poisoned"))
    }

    pub fn code(&self) -> MediaErrorCode {
        match self {
            Self::InvalidInput(_) => MediaErrorCode::InvalidInput,
            Self::Internal(_) => MediaErrorCode::InternalError,
            Self::StatePoisoned(_) => MediaErrorCode::StatePoisoned,
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Self::InvalidInput(message)
            | Self::Internal(message)
            | Self::StatePoisoned(message) => message.as_str(),
        }
    }
}

impl Display for MediaError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code().as_str(), self.message())
    }
}

impl std::error::Error for MediaError {}

impl From<String> for MediaError {
    fn from(value: String) -> Self {
        if let Some((code, detail)) = value.split_once(':') {
            if code
                .trim()
                .eq_ignore_ascii_case(MediaErrorCode::StatePoisoned.as_str())
            {
                return Self::state_poisoned(detail.trim().to_string());
            }
        }
        Self::internal(value)
    }
}

impl From<&str> for MediaError {
    fn from(value: &str) -> Self {
        Self::internal(value.to_string())
    }
}

impl From<MediaError> for MediaCommandError {
    fn from(value: MediaError) -> Self {
        Self {
            code: value.code().as_str(),
            message: value.message().to_string(),
        }
    }
}

impl From<String> for MediaCommandError {
    fn from(value: String) -> Self {
        Self {
            code: MediaErrorCode::InternalError.as_str(),
            message: value,
        }
    }
}

impl From<&str> for MediaCommandError {
    fn from(value: &str) -> Self {
        Self {
            code: MediaErrorCode::InternalError.as_str(),
            message: value.to_string(),
        }
    }
}

impl From<MediaError> for String {
    fn from(value: MediaError) -> Self {
        value.to_string()
    }
}

pub type MediaResult<T> = Result<T, MediaError>;
