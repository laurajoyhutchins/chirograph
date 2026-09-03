use std::collections::BTreeSet;

use chirograph_core::graph_json::{GraphJsonV1, GraphNodeRefV1};
use serde::{Deserialize, Serialize};

use crate::model::{GoldenFindingV1, GoldenV1};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CaseStatus {
    ExecutionFailure,
    InvalidOutput,
    Scored,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RatioCounts {
    pub numerator: u64,
    pub denominator: u64,
    pub ratio: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaseScore {
    pub contract_precision: RatioCounts,
    pub contract_recall: RatioCounts,
    pub contract_f1: Option<f64>,
    pub false_contract_rate: RatioCounts,
    pub contract_inflation: f64,
    pub authority_correctness: RatioCounts,
    pub relationship_precision: RatioCounts,
    pub relationship_recall: RatioCounts,
    pub lifecycle_correctness: Option<RatioCounts>,
    pub finding_precision: RatioCounts,
    pub finding_recall: RatioCounts,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaseResult {
    pub id: String,
    pub repository: String,
    pub scenario: String,
    pub status: CaseStatus,
    pub score: Option<CaseScore>,
    pub diagnostics: Vec<String>,
}

pub fn score_case(golden: &GoldenV1, observed: &GraphJsonV1) -> CaseScore {
    let golden_contracts = golden
        .contracts
        .iter()
        .map(|contract| contract.id.as_str())
        .collect::<BTreeSet<_>>();
    let observed_contracts = observed
        .contracts
        .iter()
        .map(|contract| contract.id.as_str())
        .collect::<BTreeSet<_>>();
    let matched_contracts = golden_contracts
        .intersection(&observed_contracts)
        .count() as u64;
    let false_contracts = observed_contracts
        .difference(&golden_contracts)
        .count() as u64;

    let contract_precision = ratio(matched_contracts, observed_contracts.len() as u64);
    let contract_recall = ratio(matched_contracts, golden_contracts.len() as u64);
    let false_contract_rate = ratio(false_contracts, observed_contracts.len() as u64);
    let contract_f1 = f1(contract_precision.ratio, contract_recall.ratio);
    let contract_inflation = inflation(observed_contracts.len(), golden_contracts.len());

    let golden_authority = golden
        .authority_claims
        .iter()
        .map(|claim| {
            (
                claim.contract.as_str(),
                claim.facet.as_str(),
                claim.representation.as_str(),
            )
        })
        .collect::<BTreeSet<_>>();
    let observed_authority = observed
        .authority_claims
        .iter()
        .map(|claim| {
            (
                claim.contract.as_str(),
                claim.facet.as_str(),
                claim.representation.as_str(),
            )
        })
        .collect::<BTreeSet<_>>();
    let authority_matches = golden_authority
        .intersection(&observed_authority)
        .count() as u64;
    let authority_correctness = ratio(authority_matches, golden_authority.len() as u64);

    let golden_relations = golden
        .relationships
        .iter()
        .map(|relation| relation_key(&relation.from, &relation.kind, &relation.to))
        .collect::<BTreeSet<_>>();
    let observed_relations = observed
        .relations
        .iter()
        .map(|relation| relation_key(&relation.from, &relation.kind, &relation.to))
        .collect::<BTreeSet<_>>();
    let relation_matches = golden_relations
        .intersection(&observed_relations)
        .count() as u64;
    let relationship_precision = ratio(relation_matches, observed_relations.len() as u64);
    let relationship_recall = ratio(relation_matches, golden_relations.len() as u64);

    let golden_findings = golden
        .expected_findings
        .iter()
        .map(|finding| match finding {
            GoldenFindingV1::ContestedClause { clause } => clause.as_str(),
        })
        .collect::<BTreeSet<_>>();
    let observed_findings = observed
        .clause_assessments
        .iter()
        .filter(|assessment| assessment.status == "contested")
        .map(|assessment| assessment.clause.as_str())
        .collect::<BTreeSet<_>>();
    let finding_matches = golden_findings
        .intersection(&observed_findings)
        .count() as u64;
    let finding_precision = ratio(finding_matches, observed_findings.len() as u64);
    let finding_recall = ratio(finding_matches, golden_findings.len() as u64);

    let mut diagnostics = Vec::new();
    let lifecycle_correctness = lifecycle_score(golden, observed, &mut diagnostics);
    known_negative_diagnostics(
        golden,
        observed,
        &golden_contracts,
        &mut diagnostics,
    );
    diagnostics.sort();
    diagnostics.dedup();

    CaseScore {
        contract_precision,
        contract_recall,
        contract_f1,
        false_contract_rate,
        contract_inflation,
        authority_correctness,
        relationship_precision,
        relationship_recall,
        lifecycle_correctness,
        finding_precision,
        finding_recall,
        diagnostics,
    }
}

fn lifecycle_score(
    golden: &GoldenV1,
    observed: &GraphJsonV1,
    diagnostics: &mut Vec<String>,
) -> Option<RatioCounts> {
    if golden.lifecycle.is_empty() {
        return None;
    }
    if observed.lifecycle.is_empty() {
        diagnostics.push("lifecycle_not_observed".to_owned());
        return None;
    }

    let golden_lifecycle = golden
        .lifecycle
        .iter()
        .map(|fact| lifecycle_key(&fact.subject, &fact.status))
        .collect::<BTreeSet<_>>();
    let observed_lifecycle = observed
        .lifecycle
        .iter()
        .map(|fact| lifecycle_key(&fact.subject, &fact.status))
        .collect::<BTreeSet<_>>();
    let matches = golden_lifecycle
        .intersection(&observed_lifecycle)
        .count() as u64;
    Some(ratio(matches, golden_lifecycle.len() as u64))
}

fn known_negative_diagnostics(
    golden: &GoldenV1,
    observed: &GraphJsonV1,
    golden_contracts: &BTreeSet<&str>,
    diagnostics: &mut Vec<String>,
) {
    if golden.non_contracts.is_empty() {
        return;
    }
    let known_negative_locators = golden
        .non_contracts
        .iter()
        .map(|item| item.locator.as_str())
        .collect::<BTreeSet<_>>();
    if observed.representations.iter().any(|representation| {
        known_negative_locators.contains(representation.locator.as_str())
            && !golden_contracts.contains(representation.contract.as_str())
    }) {
        diagnostics.push("known_non_contract_promoted".to_owned());
    }
}

fn relation_key<'a>(
    from: &'a GraphNodeRefV1,
    kind: &'a str,
    to: &'a GraphNodeRefV1,
) -> (&'a str, &'a str, &'a str, &'a str, &'a str) {
    (
        from.kind.as_str(),
        from.id.as_str(),
        kind,
        to.kind.as_str(),
        to.id.as_str(),
    )
}

fn lifecycle_key<'a>(
    subject: &'a GraphNodeRefV1,
    status: &'a str,
) -> (&'a str, &'a str, &'a str) {
    (subject.kind.as_str(), subject.id.as_str(), status)
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

fn inflation(observed: usize, golden: usize) -> f64 {
    if golden == 0 {
        observed as f64
    } else {
        observed as f64 / golden as f64
    }
}