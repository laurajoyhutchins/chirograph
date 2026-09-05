use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::model::BenchmarkCase;
use crate::score::CaseResult;

pub const BASELINE_SCHEMA: &str = "chirograph-benchmark-baseline-v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BenchmarkBaselineV1 {
    pub schema: String,
    pub cases: Vec<BaselineCaseV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BaselineCaseV1 {
    pub id: String,
    pub specimen_sha256: String,
    pub golden_sha256: String,
    pub result: CaseResult,
}

pub fn build_baseline(
    cases: &[&BenchmarkCase],
    results: &[CaseResult],
) -> Result<BenchmarkBaselineV1, String> {
    if cases.len() != results.len() {
        return Err("baseline cases and results must have equal length".to_owned());
    }

    let mut entries = cases
        .iter()
        .zip(results)
        .map(|(case, result)| {
            if case.id != result.id {
                return Err(format!(
                    "baseline case/result mismatch: {} != {}",
                    case.id, result.id
                ));
            }
            Ok(BaselineCaseV1 {
                id: case.id.clone(),
                specimen_sha256: sha256_file(&case.specimen_path)?,
                golden_sha256: sha256_file(&case.golden_path)?,
                result: result.clone(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    entries.sort_by(|left, right| left.id.cmp(&right.id));

    Ok(BenchmarkBaselineV1 {
        schema: BASELINE_SCHEMA.to_owned(),
        cases: entries,
    })
}

pub fn read_baseline(path: &Path) -> Result<BenchmarkBaselineV1, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("cannot read baseline {}: {error}", path.display()))?;
    let baseline: BenchmarkBaselineV1 = serde_json::from_str(&source)
        .map_err(|error| format!("invalid benchmark baseline {}: {error}", path.display()))?;
    if baseline.schema != BASELINE_SCHEMA {
        return Err(format!("unsupported benchmark baseline schema: {}", baseline.schema));
    }
    Ok(baseline)
}

pub fn compare_exact_baseline(
    baseline: &BenchmarkBaselineV1,
    cases: &[&BenchmarkCase],
    results: &[CaseResult],
) -> Result<(), String> {
    let current = build_baseline(cases, results)?;
    if baseline == &current {
        Ok(())
    } else {
        Err("benchmark result or corpus differs from baseline".to_owned())
    }
}

pub fn write_baseline(path: &Path, baseline: &BenchmarkBaselineV1) -> Result<(), String> {
    let encoded = serde_json::to_string_pretty(baseline)
        .map_err(|error| format!("cannot encode benchmark baseline: {error}"))?;
    fs::write(path, format!("{encoded}\n"))
        .map_err(|error| format!("cannot write baseline {}: {error}", path.display()))
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
