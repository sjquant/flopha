use thiserror::Error;

#[derive(Debug, Error)]
pub enum FlophaError {
    #[error("git error: {0}")]
    Git(#[from] git2::Error),
    #[error("failed to open repository at '{0}'")]
    RepoNotFound(String),
    #[error("remote '{0}' not found")]
    RemoteNotFound(String),
    #[error("version component '{{{0}}}' not present in pattern")]
    MissingVersionComponent(String),
}
