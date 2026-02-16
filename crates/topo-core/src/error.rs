/// Errors that can occur in Topo operations.
#[derive(Debug, thiserror::Error)]
pub enum TopoError {
    #[error("I/O error: {0}")]
    Io(String),

    #[error("scan error: {0}")]
    Scan(String),

    #[error("index error: {0}")]
    Index(String),

    #[error("score error: {0}")]
    Score(String),

    #[error("render error: {0}")]
    Render(String),

    #[error("parse error: {0}")]
    Parse(String),

    #[error("config error: {0}")]
    Config(String),
}

impl From<std::io::Error> for TopoError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err.to_string())
    }
}
