//! # certgen
//!
//! A library and CLI tool for generating certificates from ODF templates.

pub mod cli;
pub mod error;
pub mod interactive;
pub mod latex;
pub mod template;

// Re-exports
pub use cli::{Cli, Commands};
pub use error::{CertgenError, Result};
pub use latex::LatexDocument;
pub use template::CertificateData;
