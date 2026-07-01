//! # certgen
//!
//! A library and CLI tool for generating certificates from ODF templates.

pub mod error;
pub mod latex;
pub mod template;
pub mod cli;
pub mod interactive;

// Re-exports
pub use error::{CertgenError, Result};
pub use latex::LatexDocument;
pub use template::CertificateData;
pub use cli::{Cli, Commands};
