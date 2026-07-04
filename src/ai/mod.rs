//! AI explanation engine.
//!
//! Generates natural language explanations of diagnostic results
//! using external LLM services. Only structured summaries are sent
//! to the LLM — never full source code or configuration values.
//! Core diagnosis works fully offline; AI explanation gracefully
//! degrades when network is unavailable.

pub mod explain;
pub mod summary;
