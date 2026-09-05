use std::fmt::Write as _;

use crate::aggregate::{AggregateResult, BenchmarkReportV1, MacroScore};
use crate::score::{CaseResult, CaseScore, CaseStatus};

const HEADER: &str = "scope | contract P/R/F1 | false-rate | inflation | authority | relations P/R | lifecycle | findings P/R | status\n";

pub fn render_json_report(report: &BenchmarkReportV1) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(report)
}

pub fn render_human_report(report: &BenchmarkReportV1) -> String {
    let mut output = String::from(HEADER);
    for case in &report.cases {
        push_case_row(&mut output, case);
    }
    for aggregate in &report.repository_aggregates {
        push_aggregate_rows(&mut output, aggregate);
    }
    for aggregate in &report.scenario_aggregates {
        push_aggregate_rows(&mut output, aggregate);
    }
    if let Some(overall) = &report.overall {
        push_aggregate_rows(&mut output, overall);
    }
    output
}

fn push_case_row(output: &mut String, case: &CaseResult) {
    match (&case.status, &case.score) {
        (CaseStatus::Scored, Some(score)) => push_score_row(output, &case.id, score, "scored"),
        (status, _) => {
            let _ = writeln!(
                output,
                "{} | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | {}",
                case.id,
                status_name(status)
            );
        }
    }
}

fn push_aggregate_rows(output: &mut String, aggregate: &AggregateResult) {
    push_macro_row(
        output,
        &format!("{}:macro", aggregate.scope),
        &aggregate.macro_score,
    );
    if let Some(score) = &aggregate.micro_score {
        push_score_row(
            output,
            &format!("{}:micro", aggregate.scope),
            score,
            "aggregate-micro",
        );
    } else {
        let _ = writeln!(
            output,
            "{}:micro | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | aggregate-micro",
            aggregate.scope
        );
    }
}

fn push_score_row(output: &mut String, scope: &str, score: &CaseScore, status: &str) {
    let _ = writeln!(
        output,
        "{} | {}/{}/{} | {} | {:.3}x | {} | {}/{} | {} | {}/{} | {}",
        scope,
        metric(score.contract_precision.ratio),
        metric(score.contract_recall.ratio),
        metric(score.contract_f1),
        metric(score.false_contract_rate.ratio),
        score.contract_inflation,
        metric(score.authority_correctness.ratio),
        metric(score.relationship_precision.ratio),
        metric(score.relationship_recall.ratio),
        lifecycle_metric(score),
        metric(score.finding_precision.ratio),
        metric(score.finding_recall.ratio),
        status
    );
}

fn push_macro_row(output: &mut String, scope: &str, score: &MacroScore) {
    let _ = writeln!(
        output,
        "{} | {}/{}/{} | {} | {} | {} | {}/{} | {} | {}/{} | aggregate-macro",
        scope,
        metric(score.contract_precision),
        metric(score.contract_recall),
        metric(score.contract_f1),
        metric(score.false_contract_rate),
        score
            .contract_inflation
            .map(|value| format!("{value:.3}x"))
            .unwrap_or_else(|| "n/a".to_owned()),
        metric(score.authority_correctness),
        metric(score.relationship_precision),
        metric(score.relationship_recall),
        metric(score.lifecycle_correctness),
        metric(score.finding_precision),
        metric(score.finding_recall),
    );
}

fn lifecycle_metric(score: &CaseScore) -> String {
    score
        .lifecycle_correctness
        .as_ref()
        .and_then(|metric| metric.ratio)
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "n/a".to_owned())
}

fn metric(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "n/a".to_owned())
}

fn status_name(status: &CaseStatus) -> &'static str {
    match status {
        CaseStatus::ExecutionFailure => "execution-failure",
        CaseStatus::InvalidOutput => "invalid-output",
        CaseStatus::Scored => "scored",
    }
}
