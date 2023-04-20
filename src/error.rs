use thiserror::Error;

pub type Result<T> = std::result::Result<T, LFError>;

#[derive(Debug, Error)]
pub enum LFError {
    #[error("I/O error: {0}")]
    Io(#[from] #[source] std::io::Error),
    

}
