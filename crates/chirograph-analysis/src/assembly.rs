use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use chirograph_core::alignment::{
    AlignmentCatalog, AlignmentClaim, AlignmentState, ObservedRepresentation,
};
use chirograph_core::model::{
    Contract, ContractFacet, ContractGraph, ContractId, Observation, ObservationId, Representation,
    RepresentationId, RepresentationKind, Revision, Source, SourceKind,
};

use crate::{
    AnalysisError, AnalysisSourceContext, CandidateEvidence, CandidateMechanism,
    RepresentationCandidate, SemanticPath,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisAssembly {
    pub graph: ContractGraph,
    pub alignments: AlignmentCatalog,
}

pub fn assemble_contract_graph(
    context: &AnalysisSourceContext,
    candidates: &[RepresentationCandidate],
) -> Result<AnalysisAssembly, AnalysisError> {
    for candidate in candidates {
        validate_candidate_provenance(context, candidate)?;
    }

    let mut ordered = candidates.to_vec();
    ordered.sort_by(compare_candidates);

    let mut evidence = ordered
        .iter()
        .flat_map(|candidate| candidate.evidence.iter().cloned())
        .collect::<Vec<_>>();
    evidence.sort_by(compare_evidence);
    evidence.dedup();

    let observations = evidence
        .iter()
        .enumerate()
        .map(|(index, item)| Observation {
            id: observation_id(&context.namespace, index),
            source: item.source.clone(),
            revision: item.revision.clone(),
            locator: item.locator.clone(),
            fact: item.fact.clone(),
        })
        .collect::<Vec<_>>();

    let mut grouped = BTreeMap::<SemanticPath, Vec<usize>>::new();
    for (index, candidate) in ordered.iter().enumerate() {
        grouped
            .entry(candidate.semantic_path.clone())
            .or_default()
            .push(index);
    }

    let mut graph = ContractGraph {
        sources: vec![Source {
            id: context.source.clone(),
            kind: SourceKind::Repository,
            locator: format!("github:{}", context.repository),
        }],
        observations,
        ..ContractGraph::default()
    };
    let mut promoted = BTreeMap::<usize, (ContractId, RepresentationId)>::new();

    for (path, indexes) in &grouped {
        if indexes.len() != 2 {
            continue;
        }
        let source_indexes = indexes
            .iter()
            .copied()
            .filter(|index| ordered[*index].kind == RepresentationKind::SourceCode)
            .collect::<Vec<_>>();
        let schema_indexes = indexes
            .iter()
            .copied()
            .filter(|index| ordered[*index].kind == RepresentationKind::Schema)
            .collect::<Vec<_>>();
        if source_indexes.len() != 1 || schema_indexes.len() != 1 {
            continue;
        }

        let source_index = source_indexes[0];
        let schema_index = schema_indexes[0];
        let source_candidate = &ordered[source_index];
        let schema_candidate = &ordered[schema_index];
        if !eligible_source_candidate(source_candidate)
            || !eligible_schema_candidate(schema_candidate)
        {
            continue;
        }
        let (Some(source_values), Some(schema_values)) = (
            source_candidate.closed_values.as_ref(),
            schema_candidate.closed_values.as_ref(),
        ) else {
            continue;
        };
        if source_values == schema_values {
            continue;
        }

        let contract_id = ContractId::new(format!(
            "{}.{}",
            context.namespace,
            path.dotted()
        ))
        .map_err(|error| AnalysisError::Graph(format!("invalid contract id: {error:?}")))?;
        let implementation_id = RepresentationId::new(format!(
            "{}.implementation",
            contract_id.as_str()
        ))
        .map_err(|error| AnalysisError::Graph(format!("invalid representation id: {error:?}")))?;
        let schema_id = RepresentationId::new(format!("{}.schema", contract_id.as_str()))
            .map_err(|error| AnalysisError::Graph(format!("invalid representation id: {error:?}")))?;

        graph.contracts.push(Contract {
            id: contract_id.clone(),
            name: path.dotted(),
            facets: vec![ContractFacet::Structural],
        });
        graph.representations.push(Representation {
            id: implementation_id.clone(),
            contract: contract_id.clone(),
            source: context.source.clone(),
            kind: source_candidate.kind,
            locator: source_candidate.locator.clone(),
            facets: vec![ContractFacet::Structural],
        });
        graph.representations.push(Representation {
            id: schema_id.clone(),
            contract: contract_id.clone(),
            source: context.source.clone(),
            kind: schema_candidate.kind,
            locator: schema_candidate.locator.clone(),
            facets: vec![ContractFacet::Structural],
        });
        promoted.insert(source_index, (contract_id.clone(), implementation_id));
        promoted.insert(schema_index, (contract_id, schema_id));
    }

    graph.contracts.sort_by(|left, right| left.id.cmp(&right.id));
    graph.representations.sort_by(|left, right| left.id.cmp(&right.id));

    let mut alignments = AlignmentCatalog::default();
    for (index, candidate) in ordered.iter().enumerate() {
        let representation_id = promoted
            .get(&index)
            .map(|(_, representation)| representation.clone())
            .unwrap_or_else(|| observed_representation_id(&context.namespace, index));
        alignments.representations.push(ObservedRepresentation {
            id: representation_id.clone(),
            source: context.source.clone(),
            kind: candidate.kind,
            locator: candidate.locator.clone(),
        });

        if let Some((contract, _)) = promoted.get(&index) {
            alignments.claims.push(AlignmentClaim {
                representation: representation_id,
                contract: contract.clone(),
                facet: ContractFacet::Structural,
                state: AlignmentState::Confirmed,
                evidence: candidate_observation_ids(&context.namespace, candidate, &evidence),
            });
        }
    }
    alignments
        .representations
        .sort_by(|left, right| left.id.cmp(&right.id));
    alignments.claims.sort_by(|left, right| {
        (&left.representation, &left.contract, left.facet, left.state).cmp(&(
            &right.representation,
            &right.contract,
            right.facet,
            right.state,
        ))
    });

    graph
        .validate()
        .map_err(|error| AnalysisError::Graph(format!("{error:?}")))?;
    alignments
        .validate_against(&graph)
        .map_err(|error| AnalysisError::InvalidAlignment(format!("{error:?}")))?;

    Ok(AnalysisAssembly { graph, alignments })
}

fn validate_candidate_provenance(
    context: &AnalysisSourceContext,
    candidate: &RepresentationCandidate,
) -> Result<(), AnalysisError> {
    if candidate.evidence.iter().any(|evidence| {
        evidence.source != context.source || evidence.revision != context.revision
    }) {
        return Err(AnalysisError::InvalidCandidate(format!(
            "candidate {} has evidence outside explicit analysis provenance",
            candidate.qualified_local_identity
        )));
    }
    Ok(())
}

fn eligible_source_candidate(candidate: &RepresentationCandidate) -> bool {
    candidate.facets.contains(&ContractFacet::Structural)
        && candidate
            .mechanisms
            .contains(&CandidateMechanism::RustSerializedField)
        && candidate
            .mechanisms
            .contains(&CandidateMechanism::RustTypeReference)
        && candidate
            .mechanisms
            .contains(&CandidateMechanism::RustClosedValueSet)
}

fn eligible_schema_candidate(candidate: &RepresentationCandidate) -> bool {
    candidate.facets.contains(&ContractFacet::Structural)
        && candidate
            .mechanisms
            .contains(&CandidateMechanism::JsonSchemaProperty)
        && candidate
            .mechanisms
            .contains(&CandidateMechanism::JsonSchemaClosedValueSet)
}

fn candidate_observation_ids(
    namespace: &str,
    candidate: &RepresentationCandidate,
    evidence: &[CandidateEvidence],
) -> Vec<ObservationId> {
    candidate
        .evidence
        .iter()
        .map(|item| {
            let index = evidence
                .iter()
                .position(|candidate| candidate == item)
                .expect("candidate evidence was collected before assembly");
            observation_id(namespace, index)
        })
        .collect()
}

fn observation_id(namespace: &str, index: usize) -> ObservationId {
    ObservationId::new(format!("{namespace}.observation.{:04}", index + 1))
        .expect("validated namespace and ordinal form a valid observation id")
}

fn observed_representation_id(namespace: &str, index: usize) -> RepresentationId {
    RepresentationId::new(format!("{namespace}.observed.{:04}", index + 1))
        .expect("validated namespace and ordinal form a valid representation id")
}

fn compare_candidates(
    left: &RepresentationCandidate,
    right: &RepresentationCandidate,
) -> Ordering {
    left.semantic_path
        .cmp(&right.semantic_path)
        .then_with(|| kind_rank(left.kind).cmp(&kind_rank(right.kind)))
        .then_with(|| left.qualified_local_identity.cmp(&right.qualified_local_identity))
        .then_with(|| left.locator.cmp(&right.locator))
        .then_with(|| left.facets.cmp(&right.facets))
        .then_with(|| left.closed_values.cmp(&right.closed_values))
        .then_with(|| left.mechanisms.cmp(&right.mechanisms))
        .then_with(|| compare_evidence_slices(&left.evidence, &right.evidence))
}

fn compare_evidence_slices(left: &[CandidateEvidence], right: &[CandidateEvidence]) -> Ordering {
    for (left, right) in left.iter().zip(right) {
        let ordering = compare_evidence(left, right);
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    left.len().cmp(&right.len())
}

fn compare_evidence(left: &CandidateEvidence, right: &CandidateEvidence) -> Ordering {
    left.source
        .cmp(&right.source)
        .then_with(|| revision_key(&left.revision).cmp(&revision_key(&right.revision)))
        .then_with(|| left.locator.cmp(&right.locator))
        .then_with(|| left.fact.cmp(&right.fact))
}

fn revision_key(revision: &Revision) -> (u8, &str) {
    match revision {
        Revision::Exact(value) => (0, value.as_str()),
        Revision::Unversioned => (1, ""),
        Revision::Unknown => (2, ""),
    }
}

fn kind_rank(kind: RepresentationKind) -> u8 {
    match kind {
        RepresentationKind::ExecutableSurface => 0,
        RepresentationKind::SourceCode => 1,
        RepresentationKind::Schema => 2,
        RepresentationKind::TypeDefinition => 3,
        RepresentationKind::Validator => 4,
        RepresentationKind::Test => 5,
        RepresentationKind::Documentation => 6,
        RepresentationKind::Configuration => 7,
        RepresentationKind::GeneratedArtifact => 8,
        RepresentationKind::Other => 9,
    }
}
