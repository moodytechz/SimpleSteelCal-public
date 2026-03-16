use thiserror::Error;

#[derive(Debug, Error)]
pub enum SteelCalError {
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    Lookup(String),
    #[error("{0}")]
    Config(String),
    #[error("{0}")]
    Data(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

impl SteelCalError {
    #[must_use]
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation(message.into())
    }

    #[must_use]
    pub fn lookup(message: impl Into<String>) -> Self {
        Self::Lookup(message.into())
    }

    #[must_use]
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config(message.into())
    }

    #[must_use]
    pub fn data(message: impl Into<String>) -> Self {
        Self::Data(message.into())
    }

    /// Returns a clean, user-facing error string suitable for both CLI and
    /// desktop display. Unlike `Display`, this strips internal detail and
    /// provides only the actionable message.
    #[must_use]
    pub fn user_message(&self) -> String {
        match self {
            Self::Validation(msg) => msg.clone(),
            Self::Lookup(msg) => msg.clone(),
            Self::Config(msg) => format!("Configuration error: {msg}"),
            Self::Data(msg) => format!("Data error: {msg}"),
            Self::Io(err) => format!("File error: {err}"),
            Self::Json(err) => format!("Invalid data format: {err}"),
        }
    }
}
