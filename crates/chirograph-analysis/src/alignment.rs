use std::cmp::Ordering;
use std::collections::BTreeMap;

use chirograph_core::alignment::AlignmentState;
use chirograph_core::model::{ContractFacet, RepresentationKind, Revision, SourceId};

use crate::{
    AnalysisError, CandidateEvidence, CandidateMechanism, RepresentationCandidate, SemanticPath,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateKey {
    pub source: SourceId,
    pub revision: Revision,
    pub kind: RepresentationKind,
    pub qualified_local_identity: String,
    pub locator: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlignmentDecision {
    pub semantic_path: SemanticPath,
    pub candidates: Vec<CandidateKey>,
    pub facet: ContractFacet,
    pub state: AlignmentState,
    pub evidence: Vec<CandidateEvidence>,
}

pub fn align_candidates(
    candidates: &[RepresentationCandidate],
) -> Result<Vec<AlignmentDecision>, AnalysisError> {
    let mut grouped = BTreeMap::<SemanticPath, Vec<&RepresentationCandidate>>::new();
    for candidate in candidates {
        grouped
            .entry(candidate.semantic_path.clone())
            .or_default()
            .push(candidate);
    }

    let mut decisions = Vec::with_capacity(grouped.len());
    for (semantic_path, mut group) in grouped {
        group.sort_by(|left, right| compare_candidates(left, right));

        let mut keys = group
            .iter()
            .map(|candidate| candidate_key(candidate))
            .collect::<Result<Vec<_>, _>>()?;
        keys.sort_by(compare_keys);

        let mut evidence = group
            .iter()
            .flat_map(|candidate| candidate.evidence.iter().cloned())
            .collect::<Vec<_>>();
        evidence.sort_by(compare_evidence);
        evidence.dedup();

        let state = if group.len() == 2 {
            let sources = group
                .iter()
                .copied()
                .filter(|candidate| candidate.kind == RepresentationKind::SourceCode)
                .collect::<Vec<_>>();
            let schemas = group
                .iter()
                .copied()
                .filter(|candidate| candidate.kind == RepresentationKind::Schema)
                .collect::<Vec<_>>();
            if sources.len() == 1
                && schemas.len() == 1
                && eligible_source_identity(sources[0])
                && eligible_schema_identity(schemas[0])
                && compatible_provenance(sources[0], schemas[0])?
            {
                AlignmentState::Confirmed
            } else {
                AlignmentState::Unresolved
            }
        } else {
            AlignmentState::Unresolved
        };

        decisions.push(AlignmentDecision {
            semantic_path,
            candidates: keys,
            facet: ContractFacet::Structural,
            state,
            evidence,
        });
    }

    Ok(decisions)
}

fn candidate_key(candidate: &RepresentationCandidate) -> Result<CandidateKey, AnalysisError> {
    let (source, revision) = candidate_provenance(candidate)?;
    Ok(CandidateKey {
        source,
        revision,
        kind: candidate.kind,
        qualified_local_identity: candidate.qualified_local_identity.clone(),
        locator: candidate.locator.clone(),
    })
}

fn candidate_provenance(
    candidate: &RepresentationCandidate,
) -> Result<(SourceId, Revision), AnalysisError> {
    let first = candidate
        .evidence
        .first()
        .expect("validated candidates contain evidence");
    if candidate
        .evidence
        .iter()
        .any(|item| item.source != first.source || item.revision != first.revision)
    {
        return Err(AnalysisError::InvalidAlignment(format!(
            "candidate {} mixes source provenance",
            candidate.qualified_local_identity
        )));
    }
    Ok((first.source.clone(), first.revision.clone()))
}

fn compatible_provenance(
    left: &RepresentationCandidate,
    right: &RepresentationCandidate,
) -> Result<bool, AnalysisError> {
    Ok(candidate_provenance(left)? == candidate_provenance(right)?)
}

fn eligible_source_identity(candidate: &RepresentationCandidate) -> bool {
    candidate.facets.contains(&ContractFacet::Structural)
        && candidate
            .mechanisms
            .contains(&CandidateMechanism::RustSerializedField)
        && candidate
            .mechanisms
            .contains(&CandidateMechanism::RustTypeReference)
}

fn eligible_schema_identity(candidate: &RepresentationCandidate) -> bool {
    candidate.facets.contains(&ContractFacet::Structural)
        && candidate
            .mechanisms
            .contains(&CandidateMechanism::JsonSchemaProperty)
}

fn compare_candidates(left: &RepresentationCandidate, right: &RepresentationCandidate) -> Ordering {
    left.semantic_path
        .cmp(&right.semantic_path)
        .then_with(|| kind_rank(left.kind).cmp(&kind_rank(right.kind)))
        .then_with(|| {
            left.qualified_local_identity
                .cmp(&right.qualified_local_identity)
        })
        .then_with(|| left.locator.cmp(&right.locator))
        .then_with(|| left.facets.cmp(&right.facets))
        .then_with(|| left.closed_values.cmp(&right.closed_values))
        .then_with(|| left.mechanisms.cmp(&right.mechanisms))
        .then_with(|| compare_evidence_slices(&left.evidence, &right.evidence))
}

fn compare_keys(left: &CandidateKey, right: &CandidateKey) -> Ordering {
    left.source
        .cmp(&right.source)
        .then_with(|| revision_key(&left.revision).cmp(&revision_key(&right.revision)))
        .then_with(|| kind_rank(left.kind).cmp(&kind_rank(right.kind)))
        .then_with(|| {
            left.qualified_local_identity
                .cmp(&right.qualified_local_identity)
        })
        .then_with(|| left.locator.cmp(&right.locator))
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
