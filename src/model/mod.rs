//! System model data structures.
//!
//! Contains all the data types used to represent a software system's
//! structure: Bean dependency graph, auto-configuration model, and
//! configuration property model. All models use unified data structures
//! that support plugin extension.

pub mod auto_config;
pub mod bean_graph;
pub mod category;
pub mod confidence;
pub mod config;
pub mod issue;
pub mod report;
pub mod severity;
pub mod summary;
pub mod system_model;
pub mod system_overview;

// Re-export commonly used types
pub use category::Category;
pub use confidence::Confidence;
pub use issue::Issue;
pub use report::DiagnosticReport;
pub use severity::Severity;
pub use system_model::SystemModel;
pub use system_overview::SystemOverview;
