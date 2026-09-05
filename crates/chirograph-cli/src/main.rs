#![forbid(unsafe_code)]

use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use chirograph_analysis::{AnalysisSourceContext, analyze_tree};
use chirograph_core::alignment::AlignmentState;
use chirograph_core::alignment_interchange::parse_alignment_json;
use chirograph_core::evidence::parse_evidence_json;
use chirograph_core::graph_json::encode_graph_json;
use chirograph_core::model::{
    AuthorityBasis, ClauseKind, ClauseStatus, ContractFacet, ContractGraph, ContractId,
    RepresentationId, Revision,
};
use chirograph_core::query::SemanticQuery;

const HELP: &str = "Usage:\n  chirograph analyze <source-tree> --source-repository <owner/repo> --revision <40-hex|unversioned|unknown> --format graph-json\n  chirograph inspect <evidence.json>\n  chirograph contestations <evidence.json>\n  chirograph evidence <evidence.json> <contract-id>\n  chirograph authority <evidence.json> <contract-id> <facet>\n  chirograph alignment <evidence.json> <alignments.json> <representation-id>\n  chirograph --version\n  chirograph --help\n";

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
        [
            command,
            path,
            repository_flag,
            repository,
            revision_flag,
            revision,
            format_flag,
            format,
        ] if command == "analyze"
            && repository_flag == "--source-repository"
            && revision_flag == "--revision"
            && format_flag == "--format"
            && format == "graph-json" =>
        {
            analyze(Path::new(path), repository, revision)
        }
        [command, path] if command == "inspect" => inspect(Path::new(path)),
        [command, path] if command == "contestations" => contestations(Path::new(path)),
        [command, path, contract] if command == "evidence" => evidence(Path::new(path), contract),
        [command, path, contract, facet] if command == "authority" => {
            authority(Path::new(path), contract, facet)
        }
        [command, evidence_path, alignment_path, representation] if command == "alignment" => {
            alignment(
                Path::new(evidence_path),
                Path::new(alignment_path),
                representation,
            )
        }
        _ => Err(format!("invalid arguments\n\n{HELP}")),
    }
}

fn analyze(path: &Path, repository: &str, revision: &str) -> Result<String, String> {
    let revision = parse_analysis_revision(revision)?;
    let context = AnalysisSourceContext::github(repository, revision)
        .map_err(|error| format!("invalid source repository: {error:?}"))?;
    let graph = analyze_tree(path, &context)
        .map_err(|error| format!("cannot analyze source tree: {error:?}"))?;
    encode_graph_json(&graph)
        .map_err(|error| format!("cannot encode analyzed contract graph: {error:?}"))
}

fn parse_analysis_revision(value: &str) -> Result<Revision, String> {
    match value {
        "unversioned" => Ok(Revision::Unversioned),
        "unknown" => Ok(Revision::Unknown),
        value if value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) => {
            Ok(Revision::Exact(value.to_owned()))
        }
        _ => Err(format!(
            "invalid revision {value:?}: expected 40 hexadecimal characters, \"unversioned\", or \"unknown\""
        )),
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

fn evidence(path: &Path, contract: &str) -> Result<String, String> {
    let graph = read_graph(path)?;
    let contract = ContractId::new(contract)
        .map_err(|error| format!("invalid contract id {contract:?}: {error:?}"))?;
    let query = SemanticQuery::new(&graph)
        .map_err(|error| format!("cannot query contract graph: {error:?}"))?;
    let observations = query
        .evidence_for(&contract)
        .map_err(|error| format!("cannot query evidence: {error:?}"))?;

    let mut output = format!("evidence {}\n", contract.as_str());
    for observation in observations {
        output.push_str(&format!(
            "  {} source={} revision={} locator={}\n",
            observation.id.as_str(),
            observation.source.as_str(),
            revision_name(&observation.revision),
            observation.locator,
        ));
        output.push_str(&format!("    {}\n", observation.fact));
    }
    Ok(output)
}

fn authority(path: &Path, contract: &str, facet: &str) -> Result<String, String> {
    let graph = read_graph(path)?;
    let contract = ContractId::new(contract)
        .map_err(|error| format!("invalid contract id {contract:?}: {error:?}"))?;
    let facet = parse_facet(facet)?;
    let query = SemanticQuery::new(&graph)
        .map_err(|error| format!("cannot query contract graph: {error:?}"))?;
    let claims = query
        .authority_for(&contract, facet)
        .map_err(|error| format!("cannot query authority: {error:?}"))?;

    let mut output = format!("authority {} {}\n", contract.as_str(), facet_name(facet));
    for claim in claims {
        output.push_str(&format!(
            "  {} ({}) evidence={}\n",
            claim.representation.as_str(),
            authority_basis_name(claim.basis),
            claim
                .evidence
                .iter()
                .map(|id| id.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        ));
    }
    Ok(output)
}

fn alignment(
    evidence_path: &Path,
    alignment_path: &Path,
    representation: &str,
) -> Result<String, String> {
    let graph = read_graph(evidence_path)?;
    let source = fs::read_to_string(alignment_path)
        .map_err(|error| format!("cannot read {}: {error}", alignment_path.display()))?;
    let catalog = parse_alignment_json(&source, &graph)
        .map_err(|error| format!("invalid Chirograph alignments: {error:?}"))?;
    let representation = RepresentationId::new(representation)
        .map_err(|error| format!("invalid representation id {representation:?}: {error:?}"))?;
    if !catalog
        .representations
        .iter()
        .any(|candidate| candidate.id == representation)
    {
        return Err(format!(
            "unknown observed representation {:?}",
            representation.as_str()
        ));
    }

    let query = SemanticQuery::with_alignments(&graph, &catalog)
        .map_err(|error| format!("cannot query contract graph: {error:?}"))?;
    let mut output = format!("alignment {}\n", representation.as_str());
    for claim in query.alignments_for(&representation) {
        output.push_str(&format!(
            "  {} {} {} evidence={}\n",
            facet_name(claim.facet),
            claim.contract.as_str(),
            alignment_state_name(claim.state),
            claim
                .evidence
                .iter()
                .map(|id| id.as_str())
                .collect::<Vec<_>>()
                .join(", "),
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

fn revision_name(value: &Revision) -> String {
    match value {
        Revision::Exact(value) => format!("exact:{value}"),
        Revision::Unversioned => "unversioned".into(),
        Revision::Unknown => "unknown".into(),
    }
}

fn parse_facet(value: &str) -> Result<ContractFacet, String> {
    match value {
        "structural" => Ok(ContractFacet::Structural),
        "executable" => Ok(ContractFacet::Executable),
        "semantic" => Ok(ContractFacet::Semantic),
        "failure" => Ok(ContractFacet::Failure),
        "concurrency" => Ok(ContractFacet::Concurrency),
        "recovery" => Ok(ContractFacet::Recovery),
        "verification" => Ok(ContractFacet::Verification),
        _ => Err(format!("invalid contract facet {value:?}")),
    }
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

const fn alignment_state_name(value: AlignmentState) -> &'static str {
    match value {
        AlignmentState::Confirmed => "CONFIRMED",
        AlignmentState::Rejected => "REJECTED",
        AlignmentState::Unresolved => "UNRESOLVED",
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
