use chirograph_benchmark::aggregate::aggregate_report;
use chirograph_benchmark::report::{render_human_report, render_json_report};
use chirograph_benchmark::score::{
    CaseResult, CaseScore, CaseStatus, RatioCounts,
};

fn ratio(numerator: u64, denominator: u64) -> RatioCounts {
    RatioCounts {
        numerator,
        denominator,
        ratio: (denominator != 0).then_some(numerator as f64 / denominator as f64),
    }
}

fn score(matches: u64, emitted: u64, golden: u64) -> CaseScore {
    let precision = ratio(matches, emitted);
    let recall = ratio(matches, golden);
    let f1 = match (precision.ratio, recall.ratio) {
        (Some(precision), Some(recall)) if precision + recall != 0.0 => {
            Some(2.0 * precision * recall / (precision + recall))
        }
        (Some(_), Some(_)) => Some(0.0),
        _ => None,
    };
    CaseScore {
        contract_precision: precision,
        contract_recall: recall,
        contract_f1: f1,
        false_contract_rate: ratio(emitted.saturating_sub(matches), emitted),
        contract_inflation: if golden == 0 {
            emitted as f64
        } else {
            emitted as f64 / golden as f64
        },
        authority_correctness: ratio(matches, golden),
        relationship_precision: ratio(matches, emitted),
        relationship_recall: ratio(matches, golden),
        lifecycle_correctness: None,
        finding_precision: ratio(matches, emitted),
        finding_recall: ratio(matches, golden),
        diagnostics: Vec::new(),
    }
}

fn case(id: &str, repository: &str, scenario: &str, score: CaseScore) -> CaseResult {
    CaseResult {
        id: id.to_owned(),
        repository: repository.to_owned(),
        scenario: scenario.to_owned(),
        status: CaseStatus::Scored,
        score: Some(score),
        diagnostics: Vec::new(),
    }
}

#[test]
fn aggregates_macro_and_micro_without_collapsing_them() {
    let report = aggregate_report(&[
        case("repo/scenario/large", "repo", "scenario", score(100, 100, 100)),
        case("repo/scenario/small", "repo", "scenario", score(0, 1, 1)),
    ]);

    assert_eq!(report.schema, "chirograph-benchmark-report-v1");
    assert_eq!(report.repository_aggregates.len(), 1);
    assert_eq!(report.scenario_aggregates.len(), 1);

    let overall = report.overall.as_ref().expect("overall aggregate");
    assert_eq!(overall.scope, "overall");
    assert_eq!(overall.macro_score.contract_precision, Some(0.5));
    assert_eq!(
        overall
            .micro_score
            .as_ref()
            .expect("micro score")
            .contract_precision,
        ratio(100, 101)
    );
    assert_ne!(
        overall.macro_score.contract_precision,
        overall
            .micro_score
            .as_ref()
            .and_then(|score| score.contract_precision.ratio)
    );
}

#[test]
fn reports_are_deterministic_and_keep_failures_visible() {
    let failed = CaseResult {
        id: "zeta/scenario/failure".to_owned(),
        repository: "zeta".to_owned(),
        scenario: "scenario".to_owned(),
        status: CaseStatus::ExecutionFailure,
        score: None,
        diagnostics: vec!["process_failed".to_owned()],
    };
    let scored = case("alpha/scenario/pass", "alpha", "scenario", score(1, 1, 1));
    let report = aggregate_report(&[failed, scored]);

    let human = render_human_report(&report);
    assert!(human.starts_with(
        "scope | contract P/R/F1 | false-rate | inflation | authority | relations P/R | lifecycle | findings P/R | status\n"
    ));
    let alpha = human.find("alpha/scenario/pass").expect("alpha row");
    let zeta = human.find("zeta/scenario/failure").expect("zeta row");
    assert!(alpha < zeta, "case rows must be lexical");
    assert!(human.contains("execution-failure"));

    let json = render_json_report(&report).expect("JSON report");
    let decoded: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    assert_eq!(decoded["schema"], "chirograph-benchmark-report-v1");
    assert_eq!(decoded["cases"][0]["id"], "alpha/scenario/pass");
    assert_eq!(decoded["cases"][1]["id"], "zeta/scenario/failure");
    assert!(!json.contains("composite_score"));
}
