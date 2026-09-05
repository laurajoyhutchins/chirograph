use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use chirograph_core::graph_json::{GRAPH_JSON_SCHEMA, GraphJsonV1};

use crate::model::BenchmarkCase;
use crate::score::{CaseResult, CaseStatus, score_case};

const MAX_STDERR_BYTES: usize = 4096;

pub fn run_case(case: &BenchmarkCase, chirograph_bin: &Path) -> CaseResult {
    let output = Command::new(chirograph_bin)
        .arg("analyze")
        .arg(&case.fixture_dir)
        .arg("--source-repository")
        .arg(&case.specimen.upstream.repository)
        .arg("--revision")
        .arg(&case.specimen.upstream.revision)
        .arg("--format")
        .arg("graph-json")
        .output();

    let output = match output {
        Ok(output) => output,
        Err(error) => {
            return result(
                case,
                CaseStatus::ExecutionFailure,
                None,
                vec![format!("process_spawn:{error}")],
            );
        }
    };

    let mut diagnostics = stderr_diagnostics(&output.stderr);
    if !output.status.success() {
        diagnostics.insert(0, format!("process_exit:{}", exit_code(&output.status)));
        return result(case, CaseStatus::ExecutionFailure, None, diagnostics);
    }

    let observed = match serde_json::from_slice::<GraphJsonV1>(&output.stdout) {
        Ok(observed) => observed,
        Err(error) => {
            diagnostics.insert(0, format!("invalid_graph_json:{error}"));
            return result(case, CaseStatus::InvalidOutput, None, diagnostics);
        }
    };
    if observed.schema != GRAPH_JSON_SCHEMA {
        diagnostics.insert(0, "wrong_graph_schema".to_owned());
        return result(case, CaseStatus::InvalidOutput, None, diagnostics);
    }

    let score = score_case(&case.golden, &observed);
    result(case, CaseStatus::Scored, Some(score), diagnostics)
}

pub fn resolve_chirograph_bin(explicit: Option<&Path>) -> Result<PathBuf, String> {
    if let Some(path) = explicit {
        return Ok(path.to_path_buf());
    }
    if let Some(path) = nonempty_env("CHIROGRAPH_BIN") {
        return Ok(PathBuf::from(path));
    }

    let workspace = workspace_root()?;
    let binary = workspace
        .join("target")
        .join("debug")
        .join(chirograph_binary_name());
    if !binary.is_file() {
        let status = Command::new("cargo")
            .args(["build", "--quiet", "-p", "chirograph-cli"])
            .current_dir(&workspace)
            .status()
            .map_err(|error| format!("failed to build chirograph-cli: {error}"))?;
        if !status.success() {
            return Err(format!(
                "cargo build -p chirograph-cli failed with {}",
                exit_code(&status)
            ));
        }
    }
    if !binary.is_file() {
        return Err(format!(
            "chirograph binary was not produced at {}",
            binary.display()
        ));
    }
    Ok(binary)
}

fn workspace_root() -> Result<PathBuf, String> {
    let current = std::env::current_dir()
        .map_err(|error| format!("cannot determine current directory: {error}"))?;
    for ancestor in current.ancestors() {
        let manifest = ancestor.join("Cargo.toml");
        let Ok(contents) = fs::read_to_string(&manifest) else {
            continue;
        };
        if contents.lines().any(|line| line.trim() == "[workspace]") {
            return Ok(ancestor.to_path_buf());
        }
    }
    Err("cannot locate Chirograph workspace root".to_owned())
}

fn chirograph_binary_name() -> &'static str {
    if cfg!(windows) {
        "chirograph.exe"
    } else {
        "chirograph"
    }
}

fn nonempty_env(name: &str) -> Option<OsString> {
    std::env::var_os(name).filter(|value| !value.is_empty())
}

fn result(
    case: &BenchmarkCase,
    status: CaseStatus,
    score: Option<crate::score::CaseScore>,
    diagnostics: Vec<String>,
) -> CaseResult {
    CaseResult {
        id: case.id.clone(),
        repository: case.repository.clone(),
        scenario: case.scenario.clone(),
        status,
        score,
        diagnostics,
    }
}

fn stderr_diagnostics(stderr: &[u8]) -> Vec<String> {
    if stderr.is_empty() {
        return Vec::new();
    }
    let truncated = stderr.len() > MAX_STDERR_BYTES;
    let end = stderr.len().min(MAX_STDERR_BYTES);
    let text = String::from_utf8_lossy(&stderr[..end]);
    let text = text.trim();
    if text.is_empty() {
        return Vec::new();
    }
    let suffix = if truncated { ":truncated" } else { "" };
    vec![format!("stderr{suffix}:{text}")]
}

fn exit_code(status: &std::process::ExitStatus) -> String {
    status
        .code()
        .map(|code| code.to_string())
        .unwrap_or_else(|| "signal".to_owned())
}
