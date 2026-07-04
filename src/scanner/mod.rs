//! Project type detection and dependency analysis.
//!
//! The scanner module auto-detects the build tool (Maven/Gradle),
//! Spring Boot version, and project dependencies from the target
//! project directory.

pub mod gradle;
pub mod maven;

use crate::error::DoctorResult;
use crate::model::system_overview::SystemOverview;
use std::path::Path;

/// Detects and scans a project, returning a system overview.
pub fn scan_project(project_path: &Path) -> DoctorResult<SystemOverview> {
    if maven::is_maven_project(project_path) {
        maven::scan(project_path)
    } else if gradle::is_gradle_project(project_path) {
        gradle::scan(project_path)
    } else {
        Err(crate::error::DoctorError::ProjectNotFound(project_path.display().to_string()))
    }
}
