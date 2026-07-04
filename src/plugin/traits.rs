use crate::error::DoctorResult;
use crate::evidence::Evidence;
use crate::model::auto_config::AutoConfigModel;
use crate::model::bean_graph::BeanGraph;
use crate::model::config::ConfigModel;
use crate::model::{Category, Issue, SystemModel, SystemOverview};
use std::future::Future;
use std::path::Path;
use std::pin::Pin;

/// Scanner detects the technology stack of a project.
///
/// Each plugin provides a Scanner that identifies whether the target
/// project uses its technology (e.g., Spring Boot, Redis, Docker).
pub trait Scanner: Send + Sync {
    /// Human-readable scanner name.
    fn name(&self) -> &str;

    /// Returns true if this scanner recognizes the project at the given path.
    fn detect(&self, project_path: &Path) -> DoctorResult<bool>;

    /// Scan the project and return a system overview.
    fn scan(&self, project_path: &Path) -> DoctorResult<SystemOverview>;
}

/// ModelBuilder constructs structured models of the target system.
///
/// Each plugin provides technology-specific model builders that
/// produce the unified model structures (BeanGraph, AutoConfigModel, ConfigModel).
pub trait ModelBuilder: Send + Sync {
    /// Human-readable builder name.
    fn name(&self) -> &str;

    /// Build the bean dependency graph from project sources.
    fn build_bean_graph(&self, project_path: &Path) -> DoctorResult<BeanGraph>;

    /// Build the auto-configuration model.
    fn build_auto_config_model(&self, project_path: &Path) -> DoctorResult<AutoConfigModel>;

    /// Build the configuration property model.
    fn build_config_model(&self, project_path: &Path) -> DoctorResult<ConfigModel>;
}

/// EvidenceCollector gathers diagnostic facts from various sources.
///
/// Evidence is collected from source code, configuration files, and
/// (optionally) runtime endpoints like Spring Boot Actuator.
pub trait EvidenceCollector: Send + Sync {
    /// Human-readable collector name.
    fn name(&self) -> &str;

    /// Collect evidence from source code analysis.
    fn collect_source_evidence(&self, project_path: &Path) -> DoctorResult<Vec<Evidence>>;

    /// Collect evidence from configuration files.
    fn collect_config_evidence(&self, project_path: &Path) -> DoctorResult<Vec<Evidence>>;

    /// Collect evidence from runtime endpoints (optional — only if Actuator is reachable).
    /// Returns a boxed future for dyn-compatibility.
    fn collect_runtime_evidence(
        &self,
        base_url: &str,
    ) -> Pin<Box<dyn Future<Output = DoctorResult<Vec<Evidence>>> + Send + '_>>;
}

/// A single diagnostic rule that detects a specific category of issues.
pub trait DiagnosticRule: Send + Sync {
    /// Unique rule identifier (e.g., "bean-missing").
    fn id(&self) -> &str;

    /// Human-readable rule name.
    fn name(&self) -> &str;

    /// The category of issues this rule detects.
    fn category(&self) -> Category;

    /// Execute the diagnostic rule against the system model and collected evidence.
    ///
    /// Returns a list of issues found. Returns an empty Vec if no issues detected.
    fn diagnose(&self, model: &SystemModel, evidence: &[Evidence]) -> DoctorResult<Vec<Issue>>;
}

/// RuleProvider supplies a set of diagnostic rules.
///
/// Each plugin implements RuleProvider to contribute technology-specific
/// diagnostic rules. The built-in Spring Boot rules are provided by the
/// default plugin via this trait.
pub trait RuleProvider: Send + Sync {
    /// Human-readable provider name.
    fn name(&self) -> &str;

    /// Returns all diagnostic rules provided by this plugin.
    fn rules(&self) -> Vec<Box<dyn DiagnosticRule>>;
}
