use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::model::{BenchmarkCase, FixtureFileV1, parse_golden_yaml, parse_specimen_yaml};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusError {
    message: String,
}

impl CorpusError {
    fn invalid(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for CorpusError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for CorpusError {}

pub fn discover_corpus(root: &Path) -> Result<Vec<BenchmarkCase>, CorpusError> {
    let mut cases = Vec::new();
    for repository in directory_children(
        root,
        &["README.md", "baseline.json", "provenance.schema.json"],
    )? {
        let repository_name = file_name(&repository)?;
        for scenario in directory_children(&repository, &[])? {
            let scenario_name = file_name(&scenario)?;
            for case_root in directory_children(&scenario, &[])? {
                let case_name = file_name(&case_root)?;
                cases.push(load_case(
                    &case_root,
                    &repository_name,
                    &scenario_name,
                    &case_name,
                )?);
            }
        }
    }
    cases.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(cases)
}

fn load_case(
    root: &Path,
    repository: &str,
    scenario: &str,
    case_name: &str,
) -> Result<BenchmarkCase, CorpusError> {
    validate_case_entries(root)?;

    let specimen_path = root.join("specimen.yaml");
    let golden_path = root.join("golden.yaml");
    let fixture_dir = root.join("fixture");
    if !fixture_dir.is_dir() {
        return Err(CorpusError::invalid(format!(
            "case {} is missing fixture/",
            root.display()
        )));
    }

    let specimen_text = fs::read_to_string(&specimen_path)
        .map_err(|error| CorpusError::invalid(format!("{}: {error}", specimen_path.display())))?;
    let specimen = parse_specimen_yaml(&specimen_text)
        .map_err(|error| CorpusError::invalid(format!("{}: {error}", specimen_path.display())))?;
    let golden_text = fs::read_to_string(&golden_path)
        .map_err(|error| CorpusError::invalid(format!("{}: {error}", golden_path.display())))?;
    let golden = parse_golden_yaml(&golden_text)
        .map_err(|error| CorpusError::invalid(format!("{}: {error}", golden_path.display())))?;

    let expected_id = format!("{repository}/{scenario}/{case_name}");
    if specimen.id != expected_id
        || specimen.repository != repository
        || specimen.scenario != scenario
    {
        return Err(CorpusError::invalid(format!(
            "case metadata does not match path {expected_id}"
        )));
    }

    validate_fixture_bytes(root, &specimen.files)?;

    Ok(BenchmarkCase {
        id: expected_id,
        repository: repository.to_owned(),
        scenario: scenario.to_owned(),
        root: root.to_path_buf(),
        fixture_dir,
        specimen_path,
        golden_path,
        specimen,
        golden,
    })
}

fn validate_case_entries(root: &Path) -> Result<(), CorpusError> {
    let allowed = BTreeSet::from([
        "fixture",
        "golden.yaml",
        "provenance.json",
        "specimen.yaml",
    ]);
    let entries = read_dir_sorted(root)?;
    for entry in entries {
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| CorpusError::invalid("case path must be valid UTF-8"))?;
        if !allowed.contains(name.as_str()) {
            return Err(CorpusError::invalid(format!(
                "case {} contains executable or unsupported entry {name}",
                root.display()
            )));
        }
    }
    for required in ["fixture", "golden.yaml", "specimen.yaml"] {
        if !root.join(required).exists() {
            return Err(CorpusError::invalid(format!(
                "case {} is missing {required}",
                root.display()
            )));
        }
    }
    Ok(())
}

fn validate_fixture_bytes(root: &Path, files: &[FixtureFileV1]) -> Result<(), CorpusError> {
    let declared = files
        .iter()
        .map(|file| (file.fixture_path.as_str(), file))
        .collect::<BTreeMap<_, _>>();
    let actual_paths = fixture_files(&root.join("fixture"))?;

    if actual_paths.len() != declared.len() {
        return Err(CorpusError::invalid(format!(
            "case {} fixture declaration count does not match committed files",
            root.display()
        )));
    }

    for path in actual_paths {
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| CorpusError::invalid(format!("{}: {error}", path.display())))?;
        if metadata.file_type().is_symlink() {
            return Err(CorpusError::invalid(format!(
                "fixture symlinks are not allowed: {}",
                path.display()
            )));
        }
        let relative = normalized_relative(root, &path)?;
        let declared_file = declared
            .get(relative.as_str())
            .ok_or_else(|| CorpusError::invalid(format!("undeclared fixture file: {relative}")))?;
        let bytes = fs::read(&path)
            .map_err(|error| CorpusError::invalid(format!("{}: {error}", path.display())))?;
        let digest = format!("{:x}", Sha256::digest(&bytes));
        if digest != declared_file.sha256.to_ascii_lowercase() {
            return Err(CorpusError::invalid(format!(
                "fixture SHA-256 mismatch for {relative}"
            )));
        }
    }

    for fixture_path in declared.keys() {
        if !root.join(fixture_path).is_file() {
            return Err(CorpusError::invalid(format!(
                "declared fixture is missing: {fixture_path}"
            )));
        }
    }
    Ok(())
}

fn fixture_files(root: &Path) -> Result<Vec<PathBuf>, CorpusError> {
    let mut output = Vec::new();
    collect_fixture_files(root, &mut output)?;
    output.sort();
    Ok(output)
}

fn collect_fixture_files(root: &Path, output: &mut Vec<PathBuf>) -> Result<(), CorpusError> {
    for entry in read_dir_sorted(root)? {
        let file_type = entry.file_type().map_err(|error| {
            CorpusError::invalid(format!("{}: {error}", entry.path().display()))
        })?;
        if file_type.is_symlink() {
            return Err(CorpusError::invalid(format!(
                "fixture symlinks are not allowed: {}",
                entry.path().display()
            )));
        }
        if file_type.is_dir() {
            collect_fixture_files(&entry.path(), output)?;
        } else if file_type.is_file() {
            output.push(entry.path());
        } else {
            return Err(CorpusError::invalid(format!(
                "unsupported fixture entry: {}",
                entry.path().display()
            )));
        }
    }
    Ok(())
}

fn directory_children(root: &Path, allowed_files: &[&str]) -> Result<Vec<PathBuf>, CorpusError> {
    let mut output = Vec::new();
    for entry in read_dir_sorted(root)? {
        let file_type = entry.file_type().map_err(|error| {
            CorpusError::invalid(format!("{}: {error}", entry.path().display()))
        })?;
        if file_type.is_dir() {
            output.push(entry.path());
            continue;
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| CorpusError::invalid("corpus path must be valid UTF-8"))?;
        if !allowed_files.contains(&name.as_str()) {
            return Err(CorpusError::invalid(format!(
                "unsupported corpus entry: {}",
                entry.path().display()
            )));
        }
    }
    output.sort();
    Ok(output)
}

fn read_dir_sorted(root: &Path) -> Result<Vec<fs::DirEntry>, CorpusError> {
    let mut entries = fs::read_dir(root)
        .map_err(|error| CorpusError::invalid(format!("{}: {error}", root.display())))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| CorpusError::invalid(format!("{}: {error}", root.display())))?;
    entries.sort_by_key(fs::DirEntry::file_name);
    Ok(entries)
}

fn normalized_relative(root: &Path, path: &Path) -> Result<String, CorpusError> {
    let relative = path.strip_prefix(root).map_err(|_| {
        CorpusError::invalid(format!(
            "fixture path escapes case root: {}",
            path.display()
        ))
    })?;
    let mut components = Vec::new();
    for component in relative.components() {
        match component {
            Component::Normal(value) => components.push(
                value
                    .to_str()
                    .ok_or_else(|| CorpusError::invalid("fixture path must be valid UTF-8"))?,
            ),
            _ => {
                return Err(CorpusError::invalid(format!(
                    "unsafe fixture path: {}",
                    path.display()
                )));
            }
        }
    }
    Ok(components.join("/"))
}

fn file_name(path: &Path) -> Result<String, CorpusError> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .ok_or_else(|| CorpusError::invalid(format!("invalid corpus path: {}", path.display())))
}