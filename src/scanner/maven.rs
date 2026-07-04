use crate::error::{DoctorError, DoctorResult};
use crate::model::system_overview::{BuildTool, SystemOverview};
use quick_xml::Reader;
use quick_xml::events::Event;
use std::path::Path;

pub fn is_maven_project(project_path: &Path) -> bool {
    project_path.join("pom.xml").exists()
}

pub fn scan(project_path: &Path) -> DoctorResult<SystemOverview> {
    let pom_path = project_path.join("pom.xml");
    let content = std::fs::read_to_string(&pom_path)
        .map_err(|e| DoctorError::IoError { path: pom_path.display().to_string(), source: e })?;

    let mut reader = Reader::from_str(&content);
    reader.config_mut().trim_text(true);

    let mut starters: Vec<String> = Vec::new();
    let mut spring_boot_version: Option<String> = None;
    let mut java_version: Option<String> = None;
    let mut in_parent = false;
    let mut in_properties = false;
    let mut in_dependencies = false;
    let mut current_tag = String::new();
    let mut buf = Vec::new();
    let mut artifact_id = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                current_tag = name.clone();
                match name.as_str() {
                    "parent" => in_parent = true,
                    "properties" => in_properties = true,
                    "dependencies" => in_dependencies = true,
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match name.as_str() {
                    "parent" => in_parent = false,
                    "properties" => in_properties = false,
                    "dependencies" => in_dependencies = false,
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) => {
                let text = e.unescape().unwrap_or_default().to_string();
                if text.trim().is_empty() {
                    continue;
                }

                if in_parent && current_tag == "version" {
                    // Check if parent is spring-boot-starter-parent
                    if artifact_id.contains("spring-boot-starter-parent") {
                        spring_boot_version = Some(text.clone());
                    }
                }

                if in_properties {
                    if current_tag == "java.version" || current_tag == "maven.compiler.source" {
                        java_version = Some(text.clone());
                    }
                }

                if in_dependencies && current_tag == "artifactId" {
                    artifact_id = text.clone();
                    if artifact_id.starts_with("spring-boot-starter-") {
                        starters.push(artifact_id.clone());
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(DoctorError::ParseError {
                    file: pom_path.display().to_string(),
                    message: format!("XML parse error: {e}"),
                });
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(SystemOverview::new(BuildTool::Maven, spring_boot_version, java_version, starters, 1))
}
