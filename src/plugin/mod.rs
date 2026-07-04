//! Plugin system for extending Doctor with technology-specific support.
//!
//! Plugins provide Scanner, ModelBuilder, EvidenceCollector, and
//! RuleProvider implementations. Plugins are discovered by scanning
//! the default plugin directory and explicitly enabled via CLI or config.

pub mod registry;
pub mod traits;
