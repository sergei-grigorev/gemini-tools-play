use std::io;
use thiserror::Error;

/// Custom error types for the cloud-gemini application
#[derive(Error, Debug)]
pub enum AppError {
    /// Error when a required parameter is missing from a tool call
    #[error("Missing parameter: {0}")]
    MissingParameter(String),

    /// Error when a tool call function is not implemented
    #[error("Tool call function not implemented: {0}")]
    UnsupportedToolCall(String),

    /// Error when API request fails
    #[error("API request failed: {0}")]
    ApiRequestFailed(String),

    /// Error when environment variable is not set
    #[error("Environment variable not set: {0}")]
    EnvVarNotSet(String),

    /// Error when parsing API response
    #[error("Failed to parse API response: {0}")]
    ResponseParseError(String),

    /// Wrapper for reqwest errors
    #[error("HTTP request error: {0}")]
    RequestError(#[from] reqwest::Error),
    
    /// Wrapper for I/O errors
    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),
    
    /// Wrapper for JSON serialization/deserialization errors
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}
