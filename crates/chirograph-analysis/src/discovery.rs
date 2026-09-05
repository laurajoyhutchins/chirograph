use std::fs;
use std::path::{Path, PathBuf};

use crate::AnalysisError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourceFileKind {
    Rust,
    Json,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredSource {
    pub relative_path: PathBuf,
    pub kind: SourceFileKind,
}

pub fn discover_sources(root: &Path) -> Result<Vec<DiscoveredSource>, AnalysisError> {
    if !root.is_dir() {
        return Err(AnalysisError::InvalidSourceRoot(root.display().to_string()));
    }
    let mut found = Vec::new();
    walk(root, root, &mut found)?;
    found.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(found)
}

fn walk(
    root: &Path,
    directory: &Path,
    found: &mut Vec<DiscoveredSource>,
) -> Result<(), AnalysisError> {
    let mut entries = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        if file_type.is_dir() {
            walk(root, &path, found)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let kind = match path.extension().and_then(|value| value.to_str()) {
            Some("rs") => Some(SourceFileKind::Rust),
            Some("json") => Some(SourceFileKind::Json),
            _ => None,
        };
        if let Some(kind) = kind {
            let relative_path = path
                .strip_prefix(root)
                .map_err(|_| AnalysisError::InvalidSourceRoot(root.display().to_string()))?
                .to_path_buf();
            found.push(DiscoveredSource {
                relative_path,
                kind,
            });
        }
    }
    Ok(())
}
