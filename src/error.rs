use thiserror::Error;

#[derive(Debug, Error)]
pub enum FlophaError {
    #[error("git error: {0}")]
    Git(#[from] git2::Error),
    #[error("failed to open repository at '{path}'")]
    RepoNotFound {
        path: String,
        #[source]
        source: git2::Error,
    },
    #[error("remote '{name}' not found")]
    RemoteNotFound {
        name: String,
        #[source]
        source: git2::Error,
    },
    #[error("version component '{{{0}}}' not present in pattern")]
    MissingVersionComponent(String),
    #[error("invalid rule '{input}': {reason}")]
    InvalidRule { input: String, reason: String },
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid argument: {0}")]
    InvalidArgs(String),
    #[error("configuration error: {0}")]
    Config(String),
    #[error("command failed: {0}")]
    CommandFailed(String),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
