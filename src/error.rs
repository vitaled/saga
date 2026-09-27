//! Error types shared by the SAGA framework.

use std::path::PathBuf;

/// Errors produced while loading, validating or running a SAGA game.
#[derive(Debug, thiserror::Error)]
pub enum SagaError {
    #[error("failed to read `{path}`: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse YAML in `{path}`: {source}")]
    Yaml {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },

    #[error("invalid game definition: {0}")]
    Validation(ValidationErrors),

    #[error("asset error: {0}")]
    Asset(String),

    #[error("runtime error: {0}")]
    Runtime(String),

    #[error("graphics error: {0}")]
    Graphics(String),
}

/// A collection of validation problems found in a game definition.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ValidationErrors {
    pub errors: Vec<String>,
}

impl ValidationErrors {
    pub fn push(&mut self, message: impl Into<String>) {
        self.errors.push(message.into());
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn into_result(self) -> std::result::Result<(), SagaError> {
        if self.is_empty() {
            Ok(())
        } else {
            Err(SagaError::Validation(self))
        }
    }
}

impl std::fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{} problem(s) found:", self.errors.len())?;
        for error in &self.errors {
            writeln!(f, "  - {error}")?;
        }
        Ok(())
    }
}

/// Convenience result alias used across the crate.
pub type Result<T> = std::result::Result<T, SagaError>;
