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
}
