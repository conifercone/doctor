use crate::error::{DoctorError, DoctorResult};
use crate::evidence::{Evidence, EvidenceType, Reliability};
use regex::Regex;
use std::path::Path;
use walkdir::WalkDir;

/// Collect evidence from Java source code in the project.
pub fn collect(project_path: &Path) -> DoctorResult<Vec<Evidence>> {
    let mut evidence = Vec::new();
    let src_dir = project_path.join("src");

    if !src_dir.exists() {
        return Ok(evidence);
    }

    let annotation_re = Regex::new(
        r"@(Bean|Component|Service|Repository|Controller|Autowired|Qualifier|Transactional|ConditionalOn\w+|Configuration)"
    ).unwrap();

    for entry in WalkDir::new(&src_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "java"))
    {
        let content = std::fs::read_to_string(entry.path()).map_err(|e| DoctorError::IoError {
            path: entry.path().display().to_string(),
            source: e,
        })?;

        for (line_num, line) in content.lines().enumerate() {
            if let Some(caps) = annotation_re.captures(line) {
                let annotation = caps.get(1).unwrap().as_str();
                let source = format!("{}:{}", entry.path().display(), line_num + 1);
                evidence.push(Evidence::new(
                    EvidenceType::SourceCode,
                    source,
                    format!("Found @{annotation} annotation"),
                    Reliability::Confirmed,
                ));
            }
        }
    }

    Ok(evidence)
}
