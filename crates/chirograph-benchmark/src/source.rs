use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

use crate::model::{BenchmarkCase, SpecimenV1};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceError {
    Io(String),
    Git(String),
    InvalidRevision(String),
    RemoteMismatch(String),
}

impl std::fmt::Display for SourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(message) => write!(formatter, "I/O error: {message}"),
            Self::Git(message) => write!(formatter, "Git error: {message}"),
            Self::InvalidRevision(message) => write!(formatter, "invalid revision: {message}"),
            Self::RemoteMismatch(message) => write!(formatter, "source mismatch: {message}"),
        }
    }
}

impl std::error::Error for SourceError {}

pub trait SourceFetcher {
    fn fetch(&self, repository: &str, revision: &str, path: &str) -> Result<Vec<u8>, SourceError>;
}

#[derive(Debug)]
pub struct GitSourceFetcher {
    root: PathBuf,
    repositories: Mutex<BTreeMap<(String, String), PathBuf>>,
}

impl GitSourceFetcher {
    pub fn new() -> Result<Self, SourceError> {
        Ok(Self {
            root: create_temp_root()?,
            repositories: Mutex::new(BTreeMap::new()),
        })
    }

    fn repository_root(&self, repository: &str, revision: &str) -> Result<PathBuf, SourceError> {
        require_exact_revision(revision)?;
        let key = (repository.to_owned(), revision.to_ascii_lowercase());
        let mut repositories = self
            .repositories
            .lock()
            .map_err(|_| SourceError::Io("source cache lock poisoned".to_owned()))?;
        if let Some(path) = repositories.get(&key) {
            return Ok(path.clone());
        }

        let path = self.root.join(format!("repo-{}", repositories.len()));
        run_git(
            Command::new("git").arg("init").arg("--quiet").arg(&path),
            "git init",
        )?;
        run_git(
            Command::new("git")
                .arg("-C")
                .arg(&path)
                .args(["remote", "add", "origin"])
                .arg(repository_url(repository)),
            "git remote add origin",
        )?;
        run_git(
            Command::new("git")
                .arg("-C")
                .arg(&path)
                .args(["fetch", "--quiet", "--depth=1", "origin"])
                .arg(revision),
            "git fetch exact revision",
        )?;

        repositories.insert(key, path.clone());
        Ok(path)
    }
}

impl Drop for GitSourceFetcher {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl SourceFetcher for GitSourceFetcher {
    fn fetch(&self, repository: &str, revision: &str, path: &str) -> Result<Vec<u8>, SourceError> {
        let repository_root = self.repository_root(repository, revision)?;
        let object = format!("FETCH_HEAD:{path}");
        let output = Command::new("git")
            .arg("-C")
            .arg(repository_root)
            .arg("show")
            .arg(object)
            .output()
            .map_err(|error| SourceError::Git(format!("cannot execute git show: {error}")))?;
        if !output.status.success() {
            return Err(SourceError::Git(format!(
                "git show failed: {}",
                stderr_message(&output.stderr)
            )));
        }
        Ok(output.stdout)
    }
}

pub fn verify_sources(
    cases: &[BenchmarkCase],
    fetcher: &impl SourceFetcher,
) -> Result<(), SourceError> {
    for case in cases {
        require_exact_revision(&case.specimen.upstream.revision)?;
        for file in &case.specimen.files {
            let local_path = case.root.join(&file.fixture_path);
            let local = fs::read(&local_path)
                .map_err(|error| SourceError::Io(format!("{}: {error}", local_path.display())))?;
            let local_digest = sha256(&local);
            if local_digest != file.sha256.to_ascii_lowercase() {
                return Err(SourceError::RemoteMismatch(format!(
                    "{} committed fixture digest does not match specimen",
                    case.id
                )));
            }
            let remote = fetcher.fetch(
                &case.specimen.upstream.repository,
                &case.specimen.upstream.revision,
                &file.upstream_path,
            )?;
            let remote_digest = sha256(&remote);
            if remote != local || remote_digest != file.sha256.to_ascii_lowercase() {
                return Err(SourceError::RemoteMismatch(format!(
                    "{}:{} differs from {}@{}:{}",
                    case.id,
                    file.fixture_path,
                    case.specimen.upstream.repository,
                    case.specimen.upstream.revision,
                    file.upstream_path
                )));
            }
        }
    }
    Ok(())
}

pub fn refresh_sources(
    cases: &mut [BenchmarkCase],
    exact_revision: &str,
    fetcher: &impl SourceFetcher,
) -> Result<(), SourceError> {
    require_exact_revision(exact_revision)?;

    let staged = cases
        .iter()
        .map(|case| stage_refresh(case, exact_revision, fetcher))
        .collect::<Result<Vec<_>, _>>()?;

    for (case, staged_case) in cases.iter_mut().zip(staged) {
        for file in &staged_case.files {
            let path = case.root.join(&file.fixture_path);
            fs::write(&path, &file.bytes)
                .map_err(|error| SourceError::Io(format!("{}: {error}", path.display())))?;
        }
        let specimen_yaml = yaml_serde::to_string(&staged_case.specimen)
            .map_err(|error| SourceError::Io(format!("serialize specimen YAML: {error}")))?;
        fs::write(&case.specimen_path, specimen_yaml).map_err(|error| {
            SourceError::Io(format!("{}: {error}", case.specimen_path.display()))
        })?;
        case.specimen = staged_case.specimen;
    }
    Ok(())
}

#[derive(Debug)]
struct StagedCase {
    specimen: SpecimenV1,
    files: Vec<StagedFile>,
}

#[derive(Debug)]
struct StagedFile {
    fixture_path: String,
    bytes: Vec<u8>,
}

fn stage_refresh(
    case: &BenchmarkCase,
    revision: &str,
    fetcher: &impl SourceFetcher,
) -> Result<StagedCase, SourceError> {
    let mut specimen = case.specimen.clone();
    specimen.upstream.revision = revision.to_ascii_lowercase();
    let mut files = Vec::with_capacity(specimen.files.len());
    for file in &mut specimen.files {
        let bytes = fetcher.fetch(&specimen.upstream.repository, revision, &file.upstream_path)?;
        file.sha256 = sha256(&bytes);
        files.push(StagedFile {
            fixture_path: file.fixture_path.clone(),
            bytes,
        });
    }
    Ok(StagedCase { specimen, files })
}

fn require_exact_revision(value: &str) -> Result<(), SourceError> {
    if value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(SourceError::InvalidRevision(
            "revision must be exactly 40 hexadecimal characters".to_owned(),
        ))
    }
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn repository_url(repository: &str) -> String {
    if repository.contains("://") || repository.starts_with("git@") {
        repository.to_owned()
    } else {
        format!("https://github.com/{repository}.git")
    }
}

fn create_temp_root() -> Result<PathBuf, SourceError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| SourceError::Io(format!("clock before Unix epoch: {error}")))?
        .as_nanos();
    let sequence = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "chirograph-benchmark-source-{}-{now}-{sequence}",
        std::process::id()
    ));
    fs::create_dir(&root)
        .map_err(|error| SourceError::Io(format!("{}: {error}", root.display())))?;
    Ok(root)
}

fn run_git(command: &mut Command, description: &str) -> Result<(), SourceError> {
    let output = command
        .output()
        .map_err(|error| SourceError::Git(format!("cannot execute {description}: {error}")))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(SourceError::Git(format!(
            "{description} failed: {}",
            stderr_message(&output.stderr)
        )))
    }
}

fn stderr_message(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    let text = text.trim();
    if text.is_empty() {
        "no stderr".to_owned()
    } else {
        text.to_owned()
    }
}
