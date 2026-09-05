use chirograph_benchmark::baseline::{
    BaselineCaseV1, BaselineDigests, BenchmarkBaselineV1, compare_baseline_cases,
    compare_baseline_cases_complete,
};
use chirograph_benchmark::score::{CaseResult, CaseScore, CaseStatus, RatioCounts};

fn ratio(value: f64) -> RatioCounts {
    RatioCounts {
        numerator: (value * 1000.0) as u64,
        denominator: 1000,
        ratio: Some(value),
    }
}

fn unavailable() -> RatioCounts {
    RatioCounts {
        numerator: 0,
        denominator: 0,
        ratio: None,
    }
}

fn score(value: f64, false_rate: f64, inflation: f64) -> CaseScore {
    CaseScore {
        contract_precision: ratio(value),
        contract_recall: ratio(value),
        contract_f1: Some(value),
        false_contract_rate: ratio(false_rate),
        contract_inflation: inflation,
        authority_correctness: ratio(value),
        relationship_precision: ratio(value),
        relationship_recall: ratio(value),
        lifecycle_correctness: Some(ratio(value)),
        finding_precision: ratio(value),
        finding_recall: ratio(value),
        diagnostics: Vec::new(),
    }
}

fn result(status: CaseStatus, score: Option<CaseScore>) -> CaseResult {
    CaseResult {
        id: "repo/scenario/case".to_owned(),
        repository: "repo".to_owned(),
        scenario: "scenario".to_owned(),
        status,
        score,
        diagnostics: Vec::new(),
    }
}

fn baseline(status: CaseStatus, score: Option<CaseScore>) -> BenchmarkBaselineV1 {
    BenchmarkBaselineV1 {
        schema: "chirograph-benchmark-baseline-v1".to_owned(),
        cases: vec![BaselineCaseV1 {
            id: "repo/scenario/case".to_owned(),
            specimen_sha256: "a".repeat(64),
            golden_sha256: "b".repeat(64),
            result: result(status, score),
        }],
    }
}

fn digests() -> BaselineDigests {
    BaselineDigests {
        specimen_sha256: "a".repeat(64),
        golden_sha256: "b".repeat(64),
    }
}

#[test]
fn execution_failure_to_scored_is_improvement_but_scored_to_failure_is_regression() {
    let improvement = compare_baseline_cases(
        &baseline(CaseStatus::ExecutionFailure, None),
        &[(
            result(CaseStatus::Scored, Some(score(0.5, 0.0, 0.5))),
            digests(),
        )],
    )
    .expect("comparison should succeed");
    assert!(improvement.regressions.is_empty());
    assert!(
        improvement
            .improvements
            .iter()
            .any(|item| item.contains("execution-failure -> scored"))
    );

    let regression = compare_baseline_cases(
        &baseline(CaseStatus::Scored, Some(score(0.5, 0.0, 0.5))),
        &[(result(CaseStatus::InvalidOutput, None), digests())],
    )
    .expect("comparison should succeed");
    assert!(
        regression
            .regressions
            .iter()
            .any(|item| item.contains("scored -> invalid-output"))
    );
}

#[test]
fn metric_directionality_is_fail_closed() {
    let comparison = compare_baseline_cases(
        &baseline(CaseStatus::Scored, Some(score(0.8, 0.1, 0.8))),
        &[(
            result(CaseStatus::Scored, Some(score(0.7, 0.2, 0.5))),
            digests(),
        )],
    )
    .expect("comparison should succeed");

    assert!(
        comparison
            .regressions
            .iter()
            .any(|item| item.contains("contract_precision"))
    );
    assert!(
        comparison
            .regressions
            .iter()
            .any(|item| item.contains("false_contract_rate"))
    );
    assert!(
        comparison
            .regressions
            .iter()
            .any(|item| item.contains("contract_inflation"))
    );
}

#[test]
fn inflation_may_move_toward_one_and_higher_metrics_may_increase() {
    let comparison = compare_baseline_cases(
        &baseline(CaseStatus::Scored, Some(score(0.5, 0.2, 0.4))),
        &[(
            result(CaseStatus::Scored, Some(score(0.7, 0.1, 0.8))),
            digests(),
        )],
    )
    .expect("comparison should succeed");

    assert!(
        comparison.regressions.is_empty(),
        "{:?}",
        comparison.regressions
    );
}

#[test]
fn newly_observed_zero_quality_metrics_do_not_pass_as_improvements() {
    let mut previous = score(0.0, 0.0, 0.0);
    previous.contract_precision = unavailable();
    previous.relationship_precision = unavailable();
    previous.false_contract_rate = unavailable();

    let mut current = previous.clone();
    current.contract_precision = ratio(0.0);
    current.relationship_precision = ratio(0.0);
    current.false_contract_rate = ratio(1.0);

    let comparison = compare_baseline_cases(
        &baseline(CaseStatus::Scored, Some(previous)),
        &[(result(CaseStatus::Scored, Some(current)), digests())],
    )
    .expect("comparison should succeed");

    assert!(
        comparison
            .regressions
            .iter()
            .any(|item| item.contains("contract_precision"))
    );
    assert!(
        comparison
            .regressions
            .iter()
            .any(|item| item.contains("relationship_precision"))
    );
    assert!(
        comparison
            .regressions
            .iter()
            .any(|item| item.contains("false_contract_rate"))
    );
    assert!(
        !comparison
            .improvements
            .iter()
            .any(|item| item.contains("contract_precision") || item.contains("relationship_precision"))
    );
}

#[test]
fn newly_observed_zero_false_contract_rate_may_improve() {
    let mut previous = score(0.0, 0.0, 1.0);
    previous.false_contract_rate = unavailable();
    let mut current = previous.clone();
    current.false_contract_rate = ratio(0.0);

    let comparison = compare_baseline_cases(
        &baseline(CaseStatus::Scored, Some(previous)),
        &[(result(CaseStatus::Scored, Some(current)), digests())],
    )
    .expect("comparison should succeed");

    assert!(comparison.regressions.is_empty());
    assert!(
        comparison
            .improvements
            .iter()
            .any(|item| item.contains("false_contract_rate"))
    );
}

#[test]
fn full_corpus_comparison_rejects_stale_baseline_cases_but_subset_comparison_allows_them() {
    let current_result = result(CaseStatus::Scored, Some(score(0.5, 0.0, 1.0)));
    let mut reviewed = baseline(CaseStatus::Scored, Some(score(0.5, 0.0, 1.0)));
    let mut removed_result = current_result.clone();
    removed_result.id = "repo/scenario/removed-case".to_owned();
    reviewed.cases.push(BaselineCaseV1 {
        id: removed_result.id.clone(),
        specimen_sha256: "c".repeat(64),
        golden_sha256: "d".repeat(64),
        result: removed_result,
    });

    compare_baseline_cases(&reviewed, &[(current_result.clone(), digests())])
        .expect("selector-scoped comparison may use a superset baseline");

    let error = compare_baseline_cases_complete(&reviewed, &[(current_result, digests())])
        .expect_err("full corpus comparison must reject stale baseline cases");
    assert!(error.contains("absent from current benchmark corpus"));
}

#[test]
fn stale_corpus_digests_are_rejected_instead_of_compared() {
    let mut changed = digests();
    changed.golden_sha256 = "c".repeat(64);
    let error = compare_baseline_cases(
        &baseline(CaseStatus::Scored, Some(score(0.5, 0.0, 1.0))),
        &[(
            result(CaseStatus::Scored, Some(score(0.5, 0.0, 1.0))),
            changed,
        )],
    )
    .expect_err("stale golden truth must fail closed");

    assert!(error.to_string().contains("golden digest"));
}
