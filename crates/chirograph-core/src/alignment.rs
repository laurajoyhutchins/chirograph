//! Explicit, provenance-bearing semantic alignment between observed representations and contracts.
//!
//! This module deliberately separates observing a repository artifact from deciding which logical
//! contract that artifact represents. Alignment state is always explicit and evidence-backed; the
//! kernel never resolves an unresolved claim from repetition, peer agreement, or file count.

use std::collections::BTreeSet;

use crate::model::{
    ContractFacet, ContractGraph, ContractId, ModelError, ObservationId, RepresentationId,
    RepresentationKind, SourceId,
};

/// A concrete representation observed before logical contract membership has been decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedRepresentation {
    pub id: RepresentationId,
    pub source: SourceId,
    pub kind: RepresentationKind,
    pub locator: String,
}

/// The explicit state of one representation-to-contract alignment claim for one facet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AlignmentState {
    Confirmed,
    Rejected,
    Unresolved,
}

/// An evidence-backed interpretation connecting an observed representation to a contract facet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlignmentClaim {
    pub representation: RepresentationId,
    pub contract: ContractId,
    pub facet: ContractFacet,
    pub state: AlignmentState,
    pub evidence: Vec<ObservationId>,
}

/// Pre-alignment representations and the explicit claims made about them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AlignmentCatalog {
    pub representations: Vec<ObservedRepresentation>,
    pub claims: Vec<AlignmentClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlignmentError {
    InvalidGraph(ModelError),
    DuplicateRepresentation(RepresentationId),
    UnknownSource(SourceId),
    UnknownRepresentation(RepresentationId),
    UnknownContract(ContractId),
    UnknownObservation(ObservationId),
    FacetOutsideContract {
        contract: ContractId,
        facet: ContractFacet,
    },
    EvidenceRequired {
        representation: RepresentationId,
        contract: ContractId,
        facet: ContractFacet,
    },
    DuplicateClaim {
        representation: RepresentationId,
        contract: ContractId,
        facet: ContractFacet,
    },
}

impl AlignmentCatalog {
    /// Validates alignment identities and provenance against an already source-backed contract graph.
    pub fn validate_against(&self, graph: &ContractGraph) -> Result<(), AlignmentError> {
        graph.validate().map_err(AlignmentError::InvalidGraph)?;

        let source_ids: BTreeSet<_> = graph.sources.iter().map(|source| source.id.clone()).collect();
        let contract_ids: BTreeSet<_> = graph
            .contracts
            .iter()
            .map(|contract| contract.id.clone())
            .collect();
        let observation_ids: BTreeSet<_> = graph
            .observations
            .iter()
            .map(|observation| observation.id.clone())
            .collect();

        let mut representation_ids = BTreeSet::new();
        for representation in &self.representations {
            if !representation_ids.insert(representation.id.clone()) {
                return Err(AlignmentError::DuplicateRepresentation(
                    representation.id.clone(),
                ));
            }
            if !source_ids.contains(&representation.source) {
                return Err(AlignmentError::UnknownSource(representation.source.clone()));
            }
        }

        let mut claim_ids = BTreeSet::new();
        for claim in &self.claims {
            if !representation_ids.contains(&claim.representation) {
                return Err(AlignmentError::UnknownRepresentation(
                    claim.representation.clone(),
                ));
            }
            if !contract_ids.contains(&claim.contract) {
                return Err(AlignmentError::UnknownContract(claim.contract.clone()));
            }

            let contract = graph
                .contracts
                .iter()
                .find(|contract| contract.id == claim.contract)
                .expect("known contract id must resolve");
            if !contract.facets.contains(&claim.facet) {
                return Err(AlignmentError::FacetOutsideContract {
                    contract: claim.contract.clone(),
                    facet: claim.facet,
                });
            }

            if claim.evidence.is_empty() {
                return Err(AlignmentError::EvidenceRequired {
                    representation: claim.representation.clone(),
                    contract: claim.contract.clone(),
                    facet: claim.facet,
                });
            }
            for observation in &claim.evidence {
                if !observation_ids.contains(observation) {
                    return Err(AlignmentError::UnknownObservation(observation.clone()));
                }
            }

            let identity = (
                claim.representation.clone(),
                claim.contract.clone(),
                claim.facet,
            );
            if !claim_ids.insert(identity) {
                return Err(AlignmentError::DuplicateClaim {
                    representation: claim.representation.clone(),
                    contract: claim.contract.clone(),
                    facet: claim.facet,
                });
            }
        }

        Ok(())
    }

    /// Returns only the explicitly recorded state for the exact identity supplied.
    #[must_use]
    pub fn state_for(
        &self,
        representation: &RepresentationId,
        contract: &ContractId,
        facet: ContractFacet,
    ) -> Option<AlignmentState> {
        self.claims
            .iter()
            .find(|claim| {
                &claim.representation == representation
                    && &claim.contract == contract
                    && claim.facet == facet
            })
            .map(|claim| claim.state)
    }

    /// Returns claims for one observed representation in stable contract/facet/state order.
    #[must_use]
    pub fn claims_for(&self, representation: &RepresentationId) -> Vec<AlignmentClaim> {
        let mut claims: Vec<_> = self
            .claims
            .iter()
            .filter(|claim| &claim.representation == representation)
            .cloned()
            .collect();
        claims.sort_by(|left, right| {
            (&left.contract, left.facet, left.state).cmp(&(
                &right.contract,
                right.facet,
                right.state,
            ))
        });
        claims
    }
}
