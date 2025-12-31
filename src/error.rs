use thiserror::Error;

#[derive(Error, Debug)]
pub enum AkaError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] redb::DatabaseError),

    #[error("Redb error: {0}")]
    RedbError(#[from] redb::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Transaction error: {0}")]
    TransactionError(#[from] redb::TransactionError),

    #[error("Table error: {0}")]
    TableError(#[from] redb::TableError),

    #[error("Commit error: {0}")]
    CommitError(#[from] redb::CommitError),

    #[error("Storage error: {0}")]
    StorageError(#[from] redb::StorageError),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Alias not found: {0}")]
    AliasNotFound(String),

    #[error("No definition found for alias '{0}' in scope '{1}'")]
    ScopeNotFoundInAlias(String, String),

    #[error("Invalid scope path: {0}")]
    InvalidScopePath(String),

    #[error("Operation cancelled")]
    OperationCancelled,

    #[error("Unknown error: {0}")]
    Other(#[from] anyhow::Error),
}
