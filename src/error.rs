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

    #[error("LaTeX engine not found in PATH: {0}")]
    LatexEngineNotFound(String),

    #[error("LaTeX compile failed with {engine}\nstdout:\n{stdout}\nstderr:\n{stderr}")]
    LatexCompileFailed {
        engine: String,
        stdout: String,
        stderr: String,
    },

    #[error("PDF generation failed: {0}")]
    PdfGenerationFailed(String),
}

pub type Result<T> = std::result::Result<T, CertgenError>;
