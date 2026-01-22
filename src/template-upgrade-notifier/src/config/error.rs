//! Configuration error types.

use thiserror::Error;

/// Errors that can occur during configuration parsing.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Failed to read a file.
    #[error("Failed to read file '{path}': {source}")]
    IoError {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// Failed to parse TOML content.
    #[error("Failed to parse metadata.toml in '{path}': {source}")]
    TomlError {
        path: String,
        #[source]
        source: toml::de::Error,
    },

    /// Validation error in metadata.
    #[error("Validation error in '{path}': {message}")]
    ValidationError { path: String, message: String },

    /// Missing required file.
    #[error("Missing required file: {path}")]
    MissingFile { path: String },
}
