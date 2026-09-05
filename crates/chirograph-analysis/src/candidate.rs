use std::cmp::Ordering;
use std::collections::BTreeSet;

use chirograph_core::model::{ContractFacet, RepresentationKind, Revision, SourceId};

use crate::AnalysisError;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemanticPath(Vec<String>);

impl SemanticPath {
    pub fn new<I, S>(segments: I) -> Result<Self, AnalysisError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let segments = segments.into_iter().map(Into::into).collect::<Vec<_>>();
        if segments.is_empty() {
            return Err(AnalysisError::InvalidSemanticPath(
                "path must contain at least one segment".into(),
            ));
        }
        for segment in &segments {
            if segment.is_empty() || segment.trim() != segment {
                return Err(AnalysisError::InvalidSemanticPath(segment.clone()));
            }
        }
        Ok(Self(segments))
    }

    #[must_use]
    pub fn segments(&self) -> &[String] {
        &self.0
    }

    #[must_use]
    pub fn dotted(&self) -> String {
        self.0.join(".")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CandidateMechanism {
    RustSerializedField,
    RustTypeReference,
    RustClosedValueSet,
    JsonSchemaProperty,
    JsonSchemaReference,
    JsonSchemaClosedValueSet,
    ExplicitProjection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateEvidence {
    pub source: SourceId,
    pub revision: Revision,
    pub locator: String,
    pub fact: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentationCandidate {
    pub kind: RepresentationKind,
    pub qualified_local_identity: String,
    pub locator: String,
    pub facets: BTreeSet<ContractFacet>,
    pub semantic_path: SemanticPath,
    pub closed_values: Option<BTreeSet<String>>,
    pub mechanisms: BTreeSet<CandidateMechanism>,
    pub evidence: Vec<CandidateEvidence>,
}

impl RepresentationCandidate {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        kind: RepresentationKind,
        qualified_local_identity: impl Into<String>,
        locator: impl Into<String>,
        facets: BTreeSet<ContractFacet>,
        semantic_path: SemanticPath,
        closed_values: Option<BTreeSet<String>>,
        mechanisms: BTreeSet<CandidateMechanism>,
        mut evidence: Vec<CandidateEvidence>,
    ) -> Result<Self, AnalysisError> {
        let qualified_local_identity = qualified_local_identity.into();
        let locator = locator.into();
        if qualified_local_identity.is_empty()
            || qualified_local_identity.trim() != qualified_local_identity
        {
            return Err(AnalysisError::InvalidCandidate(
                "qualified local identity must be nonempty and trimmed".into(),
            ));
        }
        if locator.is_empty() || locator.trim() != locator {
            return Err(AnalysisError::InvalidCandidate(
                "locator must be nonempty and trimmed".into(),
            ));
        }
        if facets.is_empty() {
            return Err(AnalysisError::InvalidCandidate(
                "at least one facet is required".into(),
            ));
        }
        if mechanisms.is_empty() {
            return Err(AnalysisError::InvalidCandidate(
                "at least one evidence mechanism is required".into(),
            ));
        }
        if evidence.is_empty() {
            return Err(AnalysisError::InvalidCandidate(
                "at least one evidence observation is required".into(),
            ));
        }
        for item in &evidence {
            if item.locator.is_empty() || item.locator.trim() != item.locator {
                return Err(AnalysisError::InvalidCandidate(
                    "evidence locator must be nonempty and trimmed".into(),
                ));
            }
            if item.fact.is_empty() || item.fact.trim() != item.fact {
                return Err(AnalysisError::InvalidCandidate(
                    "evidence fact must be nonempty and trimmed".into(),
                ));
            }
            if let Revision::Exact(revision) = &item.revision
                && (revision.is_empty() || revision.trim() != revision)
            {
                return Err(AnalysisError::InvalidCandidate(
                    "exact evidence revision must be nonempty and trimmed".into(),
                ));
            }
        }
        if let Some(values) = &closed_values {
            if values.is_empty()
                || values
                    .iter()
                    .any(|value| value.is_empty() || value.trim() != value)
            {
                return Err(AnalysisError::InvalidCandidate(
                    "closed values must be nonempty and trimmed".into(),
                ));
            }
            let justified = mechanisms.contains(&CandidateMechanism::RustClosedValueSet)
                || mechanisms.contains(&CandidateMechanism::JsonSchemaClosedValueSet);
            if !justified {
                return Err(AnalysisError::InvalidCandidate(
                    "closed values require a closed-value-set evidence mechanism".into(),
                ));
            }
        }

        evidence.sort_by(compare_evidence);
        evidence.dedup();

        Ok(Self {
            kind,
            qualified_local_identity,
            locator,
            facets,
            semantic_path,
            closed_values,
            mechanisms,
            evidence,
        })
    }
}

fn compare_evidence(left: &CandidateEvidence, right: &CandidateEvidence) -> Ordering {
    left.source
        .as_str()
        .cmp(right.source.as_str())
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
