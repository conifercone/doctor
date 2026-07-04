//! Error types for the Doctor diagnostic engine.
//!
//! All public APIs in the Doctor crate return `DoctorResult<T>`,
//! which maps recoverable errors to `DoctorError` variants with
//! clear, actionable messages (Constitution VI).

use std::io;

/// Error type for the Doctor diagnostic engine.
///
/// Each variant carries enough context to produce an actionable
/// error message without requiring the caller to inspect sources.
#[derive(Debug, thiserror::Error)]
pub enum DoctorError {
    /// The given path does not contain a recognized project
    /// (no build file or configuration found).
    #[error(
        "project not found at `{0}`: no supported build file detected \
         (looked for pom.xml, build.gradle, build.gradle.kts, settings.gradle)"
    )]
    ProjectNotFound(String),

    /// A build file or configuration file could not be parsed.
    #[error("failed to parse `{file}`: {message}")]
    ParseError {
        /// Path to the file that failed to parse.
        file: String,
        /// Human-readable description of the parse failure.
        message: String,
    },

    /// An HTTP or network request failed.
    #[error("network request to `{url}` failed: {source}")]
    NetworkError {
        /// The URL that was being accessed.
        url: String,
        /// The underlying `reqwest` error.
        #[source]
        source: reqwest::Error,
    },

    /// A diagnostic rule failed during execution.
    #[error("rule `{rule_id}` failed: {message}")]
    RuleExecutionError {
        /// Identifier of the rule that failed.
        rule_id: String,
        /// Human-readable description of the failure.
        message: String,
    },

    /// A file I/O operation failed.
    #[error("I/O error accessing `{path}`: {source}")]
    IoError {
        /// Path that was being read from or written to.
        path: String,
        /// The underlying I/O error.
        #[source]
        source: io::Error,
    },

    /// Configuration could not be loaded or is malformed.
    #[error("configuration error: {0}")]
    ConfigError(String),

    /// A plugin failed to load or execute.
    #[error("plugin `{plugin_name}` error: {message}")]
    PluginError {
        /// Name of the plugin that failed.
        plugin_name: String,
        /// Human-readable description of the failure.
        message: String,
    },

    /// Evidence collection failed.
    #[error("evidence collection failed: {0}")]
    EvidenceError(String),
}

/// Convenience type alias for results returned by Doctor APIs.
pub type DoctorResult<T> = Result<T, DoctorError>;

// ---------------------------------------------------------------------------
// From impls — auto-convert common error types into DoctorError variants.
// These are provided alongside the `#[from]` derive where that attribute
// is sufficient; explicit impls are necessary when the variant needs
// contextual fields (e.g. `path`) that a plain `From` cannot fill.
// ---------------------------------------------------------------------------

impl From<io::Error> for DoctorError {
    fn from(source: io::Error) -> Self {
        Self::IoError { path: "<writer>".to_string(), source }
    }
}

impl From<serde_json::Error> for DoctorError {
    fn from(source: serde_json::Error) -> Self {
        Self::ConfigError(source.to_string())
    }
}

impl From<(String, io::Error)> for DoctorError {
    fn from((path, source): (String, io::Error)) -> Self {
        Self::IoError { path, source }
    }
}
