use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::model::BenchmarkCase;
use crate::score::{CaseResult, CaseScore, CaseStatus};

pub const BASELINE_SCHEMA: &str = "chirograph-benchmark-baseline-v1";
const EPSILON: f64 = 1e-12;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaselineDigests {
    pub specimen_sha256: String,
    pub golden_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BaselineComparison {
    pub regressions: Vec<String>,
    pub improvements: Vec<String>,
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
            validate_result(result)?;
            let digests = digests_for_case(case)?;
            Ok(BaselineCaseV1 {
                id: case.id.clone(),
                specimen_sha256: digests.specimen_sha256,
                golden_sha256: digests.golden_sha256,
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
    validate_baseline(&baseline)?;
    Ok(baseline)
}

pub fn compare_baseline(
    baseline: &BenchmarkBaselineV1,
    cases: &[&BenchmarkCase],
    results: &[CaseResult],
) -> Result<BaselineComparison, String> {
    if cases.len() != results.len() {
        return Err("baseline cases and results must have equal length".to_owned());
    }
    let current = cases
        .iter()
        .zip(results)
        .map(|(case, result)| {
            if case.id != result.id {
                return Err(format!(
                    "baseline case/result mismatch: {} != {}",
                    case.id, result.id
                ));
            }
            Ok((result.clone(), digests_for_case(case)?))
        })
        .collect::<Result<Vec<_>, String>>()?;
    compare_baseline_cases(baseline, &current)
}

pub fn compare_baseline_cases(
    baseline: &BenchmarkBaselineV1,
    current: &[(CaseResult, BaselineDigests)],
) -> Result<BaselineComparison, String> {
    validate_baseline(baseline)?;
    let by_id = baseline
        .cases
        .iter()
        .map(|case| (case.id.as_str(), case))
        .collect::<BTreeMap<_, _>>();
    let mut seen = BTreeSet::new();
    let mut comparison = BaselineComparison::default();

    for (result, digests) in current {
        validate_result(result)?;
        if !seen.insert(result.id.as_str()) {
            return Err(format!("duplicate current benchmark case: {}", result.id));
        }
        let previous = by_id
            .get(result.id.as_str())
            .ok_or_else(|| format!("benchmark case {} is missing from baseline", result.id))?;
        if previous.specimen_sha256 != digests.specimen_sha256 {
            return Err(format!(
                "benchmark case {} specimen digest differs from baseline",
                result.id
            ));
        }
        if previous.golden_sha256 != digests.golden_sha256 {
            return Err(format!(
                "benchmark case {} golden digest differs from baseline",
                result.id
            ));
        }
        compare_result(previous, result, &mut comparison)?;
    }

    comparison.regressions.sort();
    comparison.improvements.sort();
    Ok(comparison)
}

pub fn write_baseline(path: &Path, baseline: &BenchmarkBaselineV1) -> Result<(), String> {
    validate_baseline(baseline)?;
    let encoded = serde_json::to_string_pretty(baseline)
        .map_err(|error| format!("cannot encode benchmark baseline: {error}"))?;
    fs::write(path, format!("{encoded}\n"))
        .map_err(|error| format!("cannot write baseline {}: {error}", path.display()))
}

fn compare_result(
    baseline: &BaselineCaseV1,
    current: &CaseResult,
    comparison: &mut BaselineComparison,
) -> Result<(), String> {
    let previous_status = &baseline.result.status;
    let current_status = &current.status;
    match (previous_status, current_status) {
        (CaseStatus::Scored, CaseStatus::Scored) => {
            let previous = baseline.result.score.as_ref().ok_or_else(|| {
                format!("baseline scored case {} is missing its score", baseline.id)
            })?;
            let current_score = current.score.as_ref().ok_or_else(|| {
                format!("current scored case {} is missing its score", current.id)
            })?;
            compare_scores(&current.id, previous, current_score, comparison);
        }
        (CaseStatus::Scored, current_failure) => comparison.regressions.push(format!(
            "{} status regressed: scored -> {}",
            current.id,
            status_name(current_failure)
        )),
        (previous_failure, CaseStatus::Scored) => comparison.improvements.push(format!(
            "{} status improved: {} -> scored",
            current.id,
            status_name(previous_failure)
        )),
        _ => {}
    }
    Ok(())
}

fn compare_scores(
    case_id: &str,
    baseline: &CaseScore,
    current: &CaseScore,
    comparison: &mut BaselineComparison,
) {
    compare_higher(
        case_id,
        "contract_precision",
        baseline.contract_precision.ratio,
        current.contract_precision.ratio,
        comparison,
    );
    compare_higher(
        case_id,
        "contract_recall",
        baseline.contract_recall.ratio,
        current.contract_recall.ratio,
        comparison,
    );
    compare_higher(
        case_id,
        "contract_f1",
        baseline.contract_f1,
        current.contract_f1,
        comparison,
    );
    compare_lower(
        case_id,
        "false_contract_rate",
        baseline.false_contract_rate.ratio,
        current.false_contract_rate.ratio,
        comparison,
    );
    compare_inflation(
        case_id,
        baseline.contract_inflation,
        current.contract_inflation,
        comparison,
    );
    compare_higher(
        case_id,
        "authority_correctness",
        baseline.authority_correctness.ratio,
        current.authority_correctness.ratio,
        comparison,
    );
    compare_higher(
        case_id,
        "relationship_precision",
        baseline.relationship_precision.ratio,
        current.relationship_precision.ratio,
        comparison,
    );
    compare_higher(
        case_id,
        "relationship_recall",
        baseline.relationship_recall.ratio,
        current.relationship_recall.ratio,
        comparison,
    );
    compare_higher(
        case_id,
        "lifecycle_correctness",
        baseline
            .lifecycle_correctness
            .as_ref()
            .and_then(|metric| metric.ratio),
        current
            .lifecycle_correctness
            .as_ref()
            .and_then(|metric| metric.ratio),
        comparison,
    );
    compare_higher(
        case_id,
        "finding_precision",
        baseline.finding_precision.ratio,
        current.finding_precision.ratio,
        comparison,
    );
    compare_higher(
        case_id,
        "finding_recall",
        baseline.finding_recall.ratio,
        current.finding_recall.ratio,
        comparison,
    );
}

fn compare_higher(
    case_id: &str,
    metric: &str,
    baseline: Option<f64>,
    current: Option<f64>,
    comparison: &mut BaselineComparison,
) {
    match (baseline, current) {
        (Some(previous), Some(now)) if now + EPSILON < previous => comparison.regressions.push(
            format!("{case_id} {metric} decreased: {previous:.6} -> {now:.6}"),
        ),
        (Some(previous), Some(now)) if now > previous + EPSILON => comparison.improvements.push(
            format!("{case_id} {metric} increased: {previous:.6} -> {now:.6}"),
        ),
        (Some(_), None) => comparison
            .regressions
            .push(format!("{case_id} {metric} became unavailable")),
        (None, Some(now)) => comparison
            .improvements
            .push(format!("{case_id} {metric} became observable at {now:.6}")),
        _ => {}
    }
}

fn compare_lower(
    case_id: &str,
    metric: &str,
    baseline: Option<f64>,
    current: Option<f64>,
    comparison: &mut BaselineComparison,
) {
    match (baseline, current) {
        (Some(previous), Some(now)) if now > previous + EPSILON => comparison.regressions.push(
            format!("{case_id} {metric} increased: {previous:.6} -> {now:.6}"),
        ),
        (Some(previous), Some(now)) if now + EPSILON < previous => comparison.improvements.push(
            format!("{case_id} {metric} decreased: {previous:.6} -> {now:.6}"),
        ),
        (Some(_), None) => comparison
            .regressions
            .push(format!("{case_id} {metric} became unavailable")),
        _ => {}
    }
}

fn compare_inflation(
    case_id: &str,
    baseline: f64,
    current: f64,
    comparison: &mut BaselineComparison,
) {
    let previous_distance = (baseline - 1.0).abs();
    let current_distance = (current - 1.0).abs();
    if current_distance > previous_distance + EPSILON {
        comparison.regressions.push(format!(
            "{case_id} contract_inflation moved farther from 1.0: {baseline:.6} -> {current:.6}"
        ));
    } else if current_distance + EPSILON < previous_distance {
        comparison.improvements.push(format!(
            "{case_id} contract_inflation moved toward 1.0: {baseline:.6} -> {current:.6}"
        ));
    }
}

fn validate_baseline(baseline: &BenchmarkBaselineV1) -> Result<(), String> {
    if baseline.schema != BASELINE_SCHEMA {
        return Err(format!(
            "unsupported benchmark baseline schema: {}",
            baseline.schema
        ));
    }
    let mut ids = BTreeSet::new();
    for case in &baseline.cases {
        if case.id.is_empty() || !ids.insert(case.id.as_str()) {
            return Err(format!(
                "invalid or duplicate baseline case id: {}",
                case.id
            ));
        }
        validate_sha256(&case.specimen_sha256, "specimen", &case.id)?;
        validate_sha256(&case.golden_sha256, "golden", &case.id)?;
        if case.result.id != case.id {
            return Err(format!(
                "baseline entry/result id mismatch: {} != {}",
                case.id, case.result.id
            ));
        }
        validate_result(&case.result)?;
    }
    Ok(())
}

fn validate_result(result: &CaseResult) -> Result<(), String> {
    match (&result.status, &result.score) {
        (CaseStatus::Scored, Some(_)) => Ok(()),
        (CaseStatus::ExecutionFailure | CaseStatus::InvalidOutput, None) => Ok(()),
        (CaseStatus::Scored, None) => {
            Err(format!("scored case {} is missing its score", result.id))
        }
        (_, Some(_)) => Err(format!(
            "non-scored case {} must not carry a score",
            result.id
        )),
    }
}

fn validate_sha256(value: &str, kind: &str, case_id: &str) -> Result<(), String> {
    if value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(format!(
            "benchmark case {case_id} has invalid {kind} SHA-256"
        ))
    }
}

fn digests_for_case(case: &BenchmarkCase) -> Result<BaselineDigests, String> {
    Ok(BaselineDigests {
        specimen_sha256: sha256_file(&case.specimen_path)?,
        golden_sha256: sha256_file(&case.golden_path)?,
    })
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn status_name(status: &CaseStatus) -> &'static str {
    match status {
        CaseStatus::ExecutionFailure => "execution-failure",
        CaseStatus::InvalidOutput => "invalid-output",
        CaseStatus::Scored => "scored",
    }
}
