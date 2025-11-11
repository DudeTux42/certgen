use thiserror::Error;

#[derive(Error, Debug)]
pub enum CertgenError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("Template file not found: {0}")]
    TemplateNotFound(String),

    #[error("Placeholder '{0}' not found in template")]
    PlaceholderNotFound(String),

    #[error("Invalid template format")]
    InvalidTemplate,

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, CertgenError>;
