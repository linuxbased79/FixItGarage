use thiserror::Error;

#[derive(Debug, Error)]
pub enum FigError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("csv error: {0}")]
    Csv(String),
}
