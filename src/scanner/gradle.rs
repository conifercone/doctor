use crate::error::{DoctorError, DoctorResult};
use crate::model::system_overview::{BuildTool, SystemOverview};
use regex::Regex;
use std::path::Path;

pub fn is_gradle_project(project_path: &Path) -> bool {
    project_path.join("build.gradle").exists()
        || project_path.join("build.gradle.kts").exists()
        || project_path.join("settings.gradle").exists()
        || project_path.join("settings.gradle.kts").exists()
}

pub fn scan(project_path: &Path) -> DoctorResult<SystemOverview> {
    let mut starters: Vec<String> = Vec::new();
    let mut spring_boot_version: Option<String> = None;
    let mut java_version: Option<String> = None;
    let mut module_count: usize = 1;

    // ── 1. Check version catalog (gradle/libs.versions.toml) ──
    let catalog_path = project_path.join("gradle").join("libs.versions.toml");
    let catalog_content = if catalog_path.exists() {
        std::fs::read_to_string(&catalog_path).ok()
    } else {
        None
    };

    if let Some(ref catalog) = catalog_content {
        // Extract Spring Boot version from version catalog
        let boot_ver_re =
            Regex::new(r#"(?i)spring-?[Bb]oot-?[Vv]ersion\s*=\s*"([^"]+)""#).unwrap();
        if let Some(caps) = boot_ver_re.captures(catalog) {
            spring_boot_version = Some(caps[1].to_string());
        }

        // Extract starters from version catalog (libraries section)
        let lib_starter_re = Regex::new(
            r#"spring-boot-starter-(\w+)\s*=\s*\{[^}]*module\s*=\s*"org\.springframework\.boot:spring-boot-starter-\w+""#,
        ).unwrap();
        for caps in lib_starter_re.captures_iter(catalog) {
            starters.push(format!("spring-boot-starter-{}", &caps[1]));
        }

        // Also check for bundle references containing starters
        let bundle_re = Regex::new(r#"(?i)spring-boot-starter-(\w+)"#).unwrap();
        for caps in bundle_re.captures_iter(catalog) {
            let name = format!("spring-boot-starter-{}", &caps[1]);
            if !starters.contains(&name) {
                starters.push(name);
            }
        }
    }

    // ── 2. Check build-logic convention plugins ──
    let build_logic_dir = project_path.join("build-logic").join("src").join("main").join("kotlin");
    if build_logic_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&build_logic_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "kts" || e == "gradle") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        // Check if this convention plugin references Spring Boot
                        if content.contains("spring.boot") || content.contains("spring-boot") {
                            if spring_boot_version.is_none() {
                                // Try to find Spring Boot plugin declaration
                                let plugin_re = Regex::new(
                                    r#"(?i)id\("org\.springframework\.boot"\)\s*version\s*"([^"]+)""#,
                                ).unwrap();
                                if let Some(caps) = plugin_re.captures(&content) {
                                    spring_boot_version = Some(caps[1].to_string());
                                } else if content.contains("org.springframework.boot") {
                                    spring_boot_version =
                                        spring_boot_version.or_else(|| Some("managed".to_string()));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // ── 3. Check root build.gradle.kts ──
    let gradle_path = if project_path.join("build.gradle.kts").exists() {
        project_path.join("build.gradle.kts")
    } else if project_path.join("build.gradle").exists() {
        project_path.join("build.gradle")
    } else {
        // No root build file but settings exist — still a Gradle project
        return Ok(SystemOverview::new(
            BuildTool::Gradle,
            spring_boot_version,
            java_version,
            starters,
            module_count,
        ));
    };

    let content = std::fs::read_to_string(&gradle_path).map_err(|e| DoctorError::IoError {
        path: gradle_path.display().to_string(),
        source: e,
    })?;

    // ── 4. Parse root build.gradle.kts for Spring Boot plugin ──
    if spring_boot_version.is_none() {
        let boot_plugin_re = Regex::new(
            r#"(?i)(?:id\s*\(\s*"org\.springframework\.boot"\s*\)\s*version\s*"([^"]+)"|id\s*['"]org\.springframework\.boot['"]\s*version\s*['"]([^'"]+)['"])"#,
        ).unwrap();
        if let Some(caps) = boot_plugin_re.captures(&content) {
            spring_boot_version = Some(
                caps.get(1)
                    .or_else(|| caps.get(2))
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default(),
            );
        }

        // Check for plugin without version (managed by settings or catalog)
        if spring_boot_version.is_none() {
            let plugin_alias_re =
                Regex::new(r#"(?i)(?:alias\s*\(\s*"([^"]*spring[^"]*boot[^"]*)"|id\s*\(\s*"org\.springframework\.boot"\s*\))"#)
                    .unwrap();
            if plugin_alias_re.is_match(&content) || content.contains("org.springframework.boot") {
                spring_boot_version = Some("managed".to_string());
            }
        }
    }

    // ── 5. Extract starters from root + submodule build files ──
    let starter_re = Regex::new(
        r#"(?i)(?:implementation|compile|runtimeOnly|annotationProcessor|api)\s*\(?\s*(?:libs\.)?['"]?([^'"]*spring-boot-starter-[^'"]+)['"]?"#,
    ).unwrap();

    // Scan root build file
    for caps in starter_re.captures_iter(&content) {
        if let Some(m) = caps.get(1) {
            let dep = m.as_str().to_string();
            if let Some(artifact) = dep.rsplit(':').next() {
                if artifact.starts_with("spring-boot-starter-") && !starters.contains(&artifact.to_string()) {
                    starters.push(artifact.to_string());
                }
            }
        }
    }

    // ── 6. Scan submodule build files for multi-module projects ──
    let settings_path = project_path.join("settings.gradle.kts");
    let settings_path_groovy = project_path.join("settings.gradle");
    let settings_content = if settings_path.exists() {
        std::fs::read_to_string(&settings_path).ok()
    } else if settings_path_groovy.exists() {
        std::fs::read_to_string(&settings_path_groovy).ok()
    } else {
        None
    };

    if let Some(ref settings) = settings_content {
        let module_re = Regex::new(r#"(?i)include\s*\(\s*"([^"]+)""#).unwrap();
        let mut submodules: Vec<String> = Vec::new();
        for caps in module_re.captures_iter(settings) {
            submodules.push(caps[1].to_string());
        }
        module_count = submodules.len().max(1);

        // Scan each submodule's build.gradle.kts for starters
        for sub in &submodules {
            let sub_dir = project_path.join(sub);
            for build_file in &["build.gradle.kts", "build.gradle"] {
                let build_path = sub_dir.join(build_file);
                if build_path.exists() {
                    if let Ok(sub_content) = std::fs::read_to_string(&build_path) {
                        for caps in starter_re.captures_iter(&sub_content) {
                            if let Some(m) = caps.get(1) {
                                let dep = m.as_str().to_string();
                                if let Some(artifact) = dep.rsplit(':').last() {
                                    if artifact.starts_with("spring-boot-starter-")
                                        && !starters.contains(&artifact.to_string())
                                    {
                                        starters.push(artifact.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // ── 7. Extract Java version ──
    let java_re =
        Regex::new(r#"(?i)(?:sourceCompatibility|JavaVersion|java\.version)\s*[=:]\s*['"]?(\d+\.?\d*)['"]?"#)
            .unwrap();
    // Check catalog first
    if let Some(ref catalog) = catalog_content {
        if let Some(caps) = java_re.captures(catalog) {
            java_version = caps.get(1).map(|m| m.as_str().to_string());
        }
    }
    // Then check build files
    if java_version.is_none() {
        if let Some(caps) = java_re.captures(&content) {
            java_version = caps.get(1).map(|m| m.as_str().to_string());
        }
    }

    // Dedup starters
    starters.sort();
    starters.dedup();

    Ok(SystemOverview::new(
        BuildTool::Gradle,
        spring_boot_version,
        java_version,
        starters,
        module_count,
    ))
}
