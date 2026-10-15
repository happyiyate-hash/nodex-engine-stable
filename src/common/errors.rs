#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineError {
    Validation(String),
    Config(String),
    Network(String),
    Unsupported(String),
    Unexpected(String),
    Crypto(String),
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validation(message) => write!(f, "validation error: {message}"),
            Self::Config(message) => write!(f, "configuration error: {message}"),
            Self::Network(message) => write!(f, "network error: {message}"),
            Self::Unsupported(message) => write!(f, "unsupported operation: {message}"),
            Self::Unexpected(message) => write!(f, "unexpected error: {message}"),
            Self::Crypto(message) => write!(f, "crypto error: {message}"),
        }
    }
}

impl std::error::Error for EngineError {}
