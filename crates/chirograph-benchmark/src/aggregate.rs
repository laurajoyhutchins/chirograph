use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::score::{CaseResult, CaseScore, CaseStatus, RatioCounts};

pub const REPORT_SCHEMA: &str = "chirograph-benchmark-report-v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MacroScore {
    pub contract_precision: Option<f64>,
    pub contract_recall: Option<f64>,
    pub contract_f1: Option<f64>,
    pub false_contract_rate: Option<f64>,
    pub contract_inflation: Option<f64>,
    pub authority_correctness: Option<f64>,
    pub relationship_precision: Option<f64>,
    pub relationship_recall: Option<f64>,
    pub lifecycle_correctness: Option<f64>,
    pub finding_precision: Option<f64>,
    pub finding_recall: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AggregateResult {
    pub scope: String,
    pub macro_score: MacroScore,
    pub micro_score: Option<CaseScore>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkReportV1 {
    pub schema: String,
    pub cases: Vec<CaseResult>,
    pub repository_aggregates: Vec<AggregateResult>,
    pub scenario_aggregates: Vec<AggregateResult>,
    pub overall: Option<AggregateResult>,
}

pub fn aggregate_report(results: &[CaseResult]) -> BenchmarkReportV1 {
    let mut cases = results.to_vec();
    cases.sort_by(|left, right| left.id.cmp(&right.id));

    let mut repositories = BTreeMap::<String, Vec<&CaseResult>>::new();
    let mut scenarios = BTreeMap::<String, Vec<&CaseResult>>::new();
    for result in results {
        repositories
            .entry(result.repository.clone())
            .or_default()
            .push(result);
        scenarios
            .entry(result.scenario.clone())
            .or_default()
            .push(result);
    }

    let repository_aggregates = repositories
        .into_iter()
        .map(|(repository, cases)| aggregate_scope(format!("repository:{repository}"), &cases))
        .collect();
    let scenario_aggregates = scenarios
        .into_iter()
        .map(|(scenario, cases)| aggregate_scope(format!("scenario:{scenario}"), &cases))
        .collect();
    let overall = (!results.is_empty()).then(|| {
        let cases = results.iter().collect::<Vec<_>>();
        aggregate_scope("overall".to_owned(), &cases)
    });

    BenchmarkReportV1 {
        schema: REPORT_SCHEMA.to_owned(),
        cases,
        repository_aggregates,
        scenario_aggregates,
        overall,
    }
}

fn aggregate_scope(scope: String, results: &[&CaseResult]) -> AggregateResult {
    let scores = results
        .iter()
        .filter_map(|result| match (&result.status, &result.score) {
            (CaseStatus::Scored, Some(score)) => Some(score),
            _ => None,
        })
        .collect::<Vec<_>>();

    AggregateResult {
        scope,
        macro_score: macro_score(&scores),
        micro_score: (!scores.is_empty()).then(|| micro_score(&scores)),
    }
}

fn macro_score(scores: &[&CaseScore]) -> MacroScore {
    MacroScore {
        contract_precision: average(scores.iter().filter_map(|score| score.contract_precision.ratio)),
        contract_recall: average(scores.iter().filter_map(|score| score.contract_recall.ratio)),
        contract_f1: average(scores.iter().filter_map(|score| score.contract_f1)),
        false_contract_rate: average(
            scores
                .iter()
                .filter_map(|score| score.false_contract_rate.ratio),
        ),
        contract_inflation: average(scores.iter().map(|score| score.contract_inflation)),
        authority_correctness: average(
            scores
                .iter()
                .filter_map(|score| score.authority_correctness.ratio),
        ),
        relationship_precision: average(
            scores
                .iter()
                .filter_map(|score| score.relationship_precision.ratio),
        ),
        relationship_recall: average(
            scores
                .iter()
                .filter_map(|score| score.relationship_recall.ratio),
        ),
        lifecycle_correctness: average(scores.iter().filter_map(|score| {
            score
                .lifecycle_correctness
                .as_ref()
                .and_then(|metric| metric.ratio)
        })),
        finding_precision: average(
            scores
                .iter()
                .filter_map(|score| score.finding_precision.ratio),
        ),
        finding_recall: average(scores.iter().filter_map(|score| score.finding_recall.ratio)),
    }
}

fn micro_score(scores: &[&CaseScore]) -> CaseScore {
    let contract_precision = pool_ratio(scores, |score| &score.contract_precision);
    let contract_recall = pool_ratio(scores, |score| &score.contract_recall);
    let false_contract_rate = pool_ratio(scores, |score| &score.false_contract_rate);
    let authority_correctness = pool_ratio(scores, |score| &score.authority_correctness);
    let relationship_precision = pool_ratio(scores, |score| &score.relationship_precision);
    let relationship_recall = pool_ratio(scores, |score| &score.relationship_recall);
    let finding_precision = pool_ratio(scores, |score| &score.finding_precision);
    let finding_recall = pool_ratio(scores, |score| &score.finding_recall);
    let lifecycle_correctness = pool_lifecycle(scores);
    let emitted = contract_precision.denominator;
    let golden = contract_recall.denominator;

    CaseScore {
        contract_f1: f1(contract_precision.ratio, contract_recall.ratio),
        contract_inflation: if golden == 0 {
            emitted as f64
        } else {
            emitted as f64 / golden as f64
        },
        contract_precision,
        contract_recall,
        false_contract_rate,
        authority_correctness,
        relationship_precision,
        relationship_recall,
        lifecycle_correctness,
        finding_precision,
        finding_recall,
        diagnostics: Vec::new(),
    }
}

fn pool_ratio(
    scores: &[&CaseScore],
    metric: impl Fn(&CaseScore) -> &RatioCounts,
) -> RatioCounts {
    let (numerator, denominator) = scores.iter().fold((0_u64, 0_u64), |totals, score| {
        let value = metric(score);
        (
            totals.0.saturating_add(value.numerator),
            totals.1.saturating_add(value.denominator),
        )
    });
    ratio(numerator, denominator)
}

fn pool_lifecycle(scores: &[&CaseScore]) -> Option<RatioCounts> {
    let mut observed = false;
    let (numerator, denominator) = scores.iter().fold((0_u64, 0_u64), |totals, score| {
        let Some(value) = &score.lifecycle_correctness else {
            return totals;
        };
        observed = true;
        (
            totals.0.saturating_add(value.numerator),
            totals.1.saturating_add(value.denominator),
        )
    });
    observed.then(|| ratio(numerator, denominator))
}

fn average(values: impl Iterator<Item = f64>) -> Option<f64> {
    let (sum, count) = values.fold((0.0, 0_u64), |(sum, count), value| {
        (sum + value, count + 1)
    });
    (count != 0).then_some(sum / count as f64)
}

fn ratio(numerator: u64, denominator: u64) -> RatioCounts {
    RatioCounts {
        numerator,
        denominator,
        ratio: (denominator != 0).then_some(numerator as f64 / denominator as f64),
    }
}

fn f1(precision: Option<f64>, recall: Option<f64>) -> Option<f64> {
    let (precision, recall) = (precision?, recall?);
    if precision + recall == 0.0 {
        Some(0.0)
    } else {
        Some(2.0 * precision * recall / (precision + recall))
    }
}
