//! Global PSI index backed by sled embedded KV database.
//!
//! Stores PsiClass entries keyed by FQCN, with secondary indexes
//! for fast simple-name lookup and file-hash-based incremental updates.
//!
//! Database location: `~/.doctor/cache/psi-index/`

use crate::psi::ast::PsiClass;
use std::path::PathBuf;

/// Key prefixes for sled entries.
const CLASS_PREFIX: &str = "C:";
const IMPORT_PREFIX: &str = "I:";
const FILE_HASH_PREFIX: &str = "H:";

/// Open (or create) the sled database at the default cache path.
pub fn open_db() -> sled::Result<sled::Db> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let path = PathBuf::from(&home).join(".doctor/cache/psi-index");
    std::fs::create_dir_all(&path).ok();
    sled::open(path)
}

/// Write a PsiClass to the index.
pub fn write_class(db: &sled::Db, cls: &PsiClass) -> sled::Result<()> {
    let fqcn = &cls.fqcn;
    let json = serde_json::to_vec(cls).unwrap_or_default();

    // Primary index: FQCN → PsiClass JSON
    db.insert(class_key(fqcn), json.as_slice())?;

    // Secondary index: simple_name → FQCN (append to comma-separated list)
    let simple = cls.name.clone();
    let import_key = import_key(&simple);
    let existing = db
        .get(&import_key)?
        .map(|v| String::from_utf8_lossy(&v).to_string())
        .unwrap_or_default();

    let mut fqcns: Vec<&str> = existing.split(',').filter(|s| !s.is_empty()).collect();
    if !fqcns.contains(&fqcn.as_str()) {
        fqcns.push(fqcn);
    }
    db.insert(import_key, fqcns.join(",").as_bytes())?;

    Ok(())
}

/// Read a PsiClass from the index by FQCN.
pub fn read_class(db: &sled::Db, fqcn: &str) -> Option<PsiClass> {
    let key = class_key(fqcn);
    let value = db.get(key).ok()??;
    serde_json::from_slice(&value).ok()
}

/// Lookup FQCNs by simple class name.
pub fn lookup_by_simple_name(db: &sled::Db, simple_name: &str) -> Vec<String> {
    let key = import_key(simple_name);
    match db.get(key) {
        Ok(Some(v)) => {
            let s = String::from_utf8_lossy(&v);
            s.split(',').map(|s| s.to_string()).filter(|s| !s.is_empty()).collect()
        }
        _ => vec![],
    }
}

/// Read all PsiClass definitions from a file path (via sled FILE→FQCNs mapping).
pub fn read_classes_from_file(db: &sled::Db, file_path: &str) -> Vec<PsiClass> {
    let key = format!("F:{file_path}");
    match db.get(key.as_bytes()) {
        Ok(Some(fqcns_bytes)) => {
            let s = String::from_utf8_lossy(&fqcns_bytes);
            s.split(',').filter(|s| !s.is_empty()).filter_map(|fqcn| read_class(db, fqcn)).collect()
        }
        _ => vec![],
    }
}

/// Update the file → FQCN mapping for a class.
fn store_file_fqcn(db: &sled::Db, file_path: &str, fqcn: &str) -> sled::Result<()> {
    let key = format!("F:{file_path}");
    db.insert(key.as_bytes(), fqcn.as_bytes())?;
    Ok(())
}

/// Store file hash for incremental scan detection.
pub fn store_file_hash(db: &sled::Db, file_path: &str, hash: &str) -> sled::Result<()> {
    let key = format!("{FILE_HASH_PREFIX}{file_path}");
    db.insert(key.as_bytes(), hash.as_bytes())?;
    Ok(())
}

/// Check if a file has changed since last index (returns true if changed or new).
pub fn file_changed(db: &sled::Db, file_path: &str, content: &str) -> bool {
    let hash = compute_hash(content);
    let key = format!("{FILE_HASH_PREFIX}{file_path}");
    match db.get(key.as_bytes()) {
        Ok(Some(existing)) => {
            let existing_str = String::from_utf8_lossy(&existing);
            existing_str != hash
        }
        _ => true, // No hash stored = new file
    }
}

/// Compute SHA256 hash of a string.
pub fn compute_hash(content: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn class_key(fqcn: &str) -> Vec<u8> {
    format!("{CLASS_PREFIX}{fqcn}").into_bytes()
}

fn import_key(simple_name: &str) -> Vec<u8> {
    format!("{IMPORT_PREFIX}{simple_name}").into_bytes()
}

/// Build the full PSI index for a project: parse all Java files → write to sled.
pub fn build_index(project_path: &std::path::Path) -> Result<sled::Db, String> {
    let db = open_db().map_err(|e| format!("sled open: {e}"))?;
    let java_source_dirs =
        crate::model::bean_graph::collect_java_source_dirs(project_path)
            .map_err(|e| format!("collect dirs: {e}"))?;

    let mut total = 0usize;
    for dir in &java_source_dirs {
        if !dir.exists() {
            continue;
        }
        for entry in walkdir::WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "java"))
        {
            match crate::psi::ast::parse_file(entry.path()) {
                Ok(classes) => {
                    for cls in &classes {
                        if let Err(e) = write_class(&db, cls) {
                            eprintln!("  Warning: sled write failed for {}: {e}", cls.fqcn);
                        }
                    }
                    total += classes.len();
                }
                Err(e) => {
                    eprintln!(
                        "  Warning: PSI parse failed for {}: {e}",
                        entry.path().display()
                    );
                }
            }
        }
    }

    // Flush to disk
    db.flush().map_err(|e| format!("sled flush: {e}"))?;
    eprintln!("  PSI index: {total} classes indexed");
    Ok(db)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::psi::ast::*;
    use tempfile::tempdir;

    #[test]
    fn test_sled_write_and_read() {
        let dir = tempdir().unwrap();
        // Use temp dir for sled
        let db = sled::open(dir.path()).unwrap();

        let cls = PsiClass {
            name: "UserService".into(),
            package: "com.example".into(),
            fqcn: "com.example.UserService".into(),
            interfaces: vec![],
            annotations: vec![],
            fields: vec![],
            methods: vec![],
            file_path: "/tmp/test.java".into(),
        };

        write_class(&db, &cls).unwrap();
        let loaded = read_class(&db, "com.example.UserService").unwrap();
        assert_eq!(loaded.name, "UserService");

        let fqcns = lookup_by_simple_name(&db, "UserService");
        assert!(fqcns.contains(&"com.example.UserService".to_string()));
    }
}
