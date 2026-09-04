#![forbid(unsafe_code)]

use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use chirograph_core::evidence::parse_evidence_json;
use chirograph_core::model::{
    AuthorityBasis, ClauseKind, ClauseStatus, ContractFacet, ContractGraph,
};
use chirograph_core::query::SemanticQuery;

const HELP: &str = "Usage:\n  chirograph inspect <evidence.json>\n  chirograph contestations <evidence.json>\n  chirograph --version\n  chirograph --help\n";

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(output) => {
            print!("{output}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Vec<String>) -> Result<String, String> {
    match args.as_slice() {
        [] => Ok(HELP.into()),
        [arg] if arg == "--help" || arg == "-h" => Ok(HELP.into()),
        [arg] if arg == "--version" || arg == "-V" => {
            Ok(format!("chirograph {}\n", chirograph_core::version()))
        }
        [command, path] if command == "inspect" => inspect(Path::new(path)),
        [command, path] if command == "contestations" => contestations(Path::new(path)),
        _ => Err(format!("invalid arguments\n\n{HELP}")),
    }
}

fn read_graph(path: &Path) -> Result<ContractGraph, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    parse_evidence_json(&source).map_err(|error| format!("invalid Chirograph evidence: {error:?}"))
}

fn inspect(path: &Path) -> Result<String, String> {
    let graph = read_graph(path)?;
    render_graph(&graph).map_err(|error| format!("cannot assess contract graph: {error:?}"))
}

fn contestations(path: &Path) -> Result<String, String> {
    let graph = read_graph(path)?;
    let query = SemanticQuery::new(&graph)
        .map_err(|error| format!("cannot query contract graph: {error:?}"))?;
    let mut output = String::new();
    for assessment in query.contestations() {
        let clause = graph
            .clauses
            .iter()
            .find(|candidate| candidate.id == assessment.clause)
            .expect("validated assessment must reference a known clause");
        output.push_str(&format!(
            "{} clause {} [{}] {}\n",
            facet_name(clause.facet),
            clause.id.as_str(),
            clause_kind_name(clause.kind),
            clause_status_name(assessment.status),
        ));
        output.push_str(&format!("  {}\n", clause.statement));
        output.push_str(&format!(
            "  supports: {}\n",
            join_representation_ids(&assessment.supporting_representations)
        ));
        output.push_str(&format!(
            "  contradicts: {}\n",
            join_representation_ids(&assessment.contradicting_representations)
        ));
    }
    Ok(output)
}

fn render_graph(graph: &ContractGraph) -> Result<String, chirograph_core::model::ModelError> {
    graph.validate()?;
    let mut output = String::new();
    let mut contracts = graph.contracts.iter().collect::<Vec<_>>();
    contracts.sort_by(|left, right| left.id.cmp(&right.id));

    for contract in contracts {
        output.push_str(&format!(
            "contract {} — {}\n",
            contract.id.as_str(),
            contract.name
        ));

        let mut clauses = graph
            .clauses
            .iter()
            .filter(|clause| clause.contract == contract.id)
            .collect::<Vec<_>>();
        clauses.sort_by(|left, right| {
            facet_name(left.facet)
                .cmp(facet_name(right.facet))
                .then_with(|| left.id.cmp(&right.id))
        });

        for clause in clauses {
            let assessment = graph.assess_clause(&clause.id)?;
            output.push_str(&format!(
                "  {} clause {} [{}] {}\n",
                facet_name(clause.facet),
                clause.id.as_str(),
                clause_kind_name(clause.kind),
                clause_status_name(assessment.status),
            ));
            output.push_str(&format!("    {}\n", clause.statement));
            output.push_str(&format!(
                "    supports: {}\n",
                join_representation_ids(&assessment.supporting_representations)
            ));
            if !assessment.contradicting_representations.is_empty() {
                output.push_str(&format!(
                    "    contradicts: {}\n",
                    join_representation_ids(&assessment.contradicting_representations)
                ));
            }
        }

        let mut claims = graph
            .authority_claims
            .iter()
            .filter(|claim| claim.contract == contract.id)
            .collect::<Vec<_>>();
        claims.sort_by(|left, right| {
            facet_name(left.facet)
                .cmp(facet_name(right.facet))
                .then_with(|| left.representation.cmp(&right.representation))
        });
        for claim in claims {
            output.push_str(&format!(
                "  authority {}: {} ({})\n",
                facet_name(claim.facet),
                claim.representation.as_str(),
                authority_basis_name(claim.basis),
            ));
        }
    }

    Ok(output)
}

fn join_representation_ids(ids: &[chirograph_core::model::RepresentationId]) -> String {
    ids.iter()
        .map(|id| id.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

const fn facet_name(value: ContractFacet) -> &'static str {
    match value {
        ContractFacet::Structural => "structural",
        ContractFacet::Executable => "executable",
        ContractFacet::Semantic => "semantic",
        ContractFacet::Failure => "failure",
        ContractFacet::Concurrency => "concurrency",
        ContractFacet::Recovery => "recovery",
        ContractFacet::Verification => "verification",
    }
}

const fn clause_kind_name(value: ClauseKind) -> &'static str {
    match value {
        ClauseKind::Requirement => "requirement",
        ClauseKind::Guarantee => "guarantee",
        ClauseKind::Invariant => "invariant",
    }
}

const fn clause_status_name(value: ClauseStatus) -> &'static str {
    match value {
        ClauseStatus::Consistent => "CONSISTENT",
        ClauseStatus::Contested => "CONTESTED",
    }
}

const fn authority_basis_name(value: AuthorityBasis) -> &'static str {
    match value {
        AuthorityBasis::ExplicitDeclaration => "explicit_declaration",
        AuthorityBasis::MechanicalEnforcement => "mechanical_enforcement",
        AuthorityBasis::ObservedBehavior => "observed_behavior",
        AuthorityBasis::Documentation => "documentation",
        AuthorityBasis::Inference => "inference",
    }
}
