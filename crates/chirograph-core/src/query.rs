//! Deterministic, read-only semantic queries over validated Chirograph state.
//!
//! Queries expose only facts and interpretations already present in the graph or alignment catalog.
//! They do not rank authority, resolve contestations, infer alignment, or treat repeated evidence as
//! additional truth.

use std::collections::BTreeSet;

use crate::alignment::{AlignmentCatalog, AlignmentClaim, AlignmentError};
use crate::model::{
    AuthorityBasis, AuthorityClaim, ClauseAssessment, ClauseStatus, Contract, ContractClause,
    ContractFacet, ContractGraph, ContractId, ModelError, NodeRef, Observation, ObservationId,
    Representation, RepresentationId,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryError {
    InvalidGraph(ModelError),
    InvalidAlignment(AlignmentError),
    UnknownContract(ContractId),
}

/// A validated read-only view over one contract graph and, optionally, its pre-alignment catalog.
pub struct SemanticQuery<'a> {
    graph: &'a ContractGraph,
    alignments: Option<&'a AlignmentCatalog>,
}

impl<'a> SemanticQuery<'a> {
    pub fn new(graph: &'a ContractGraph) -> Result<Self, QueryError> {
        graph.validate().map_err(QueryError::InvalidGraph)?;
        Ok(Self {
            graph,
            alignments: None,
        })
    }

    pub fn with_alignments(
        graph: &'a ContractGraph,
        alignments: &'a AlignmentCatalog,
    ) -> Result<Self, QueryError> {
        graph.validate().map_err(QueryError::InvalidGraph)?;
        alignments
            .validate_against(graph)
            .map_err(QueryError::InvalidAlignment)?;
        Ok(Self {
            graph,
            alignments: Some(alignments),
        })
    }

    /// Returns all represented contracts ordered by stable contract identity.
    #[must_use]
    pub fn contracts(&self) -> Vec<Contract> {
        let mut contracts = self.graph.contracts.clone();
        contracts.sort_by(|left, right| left.id.cmp(&right.id));
        contracts
    }

    /// Returns post-alignment representations explicitly assigned to a contract.
    pub fn representations_for(
        &self,
        contract: &ContractId,
    ) -> Result<Vec<Representation>, QueryError> {
        self.require_contract(contract)?;
        let mut representations: Vec<_> = self
            .graph
            .representations
            .iter()
            .filter(|representation| &representation.contract == contract)
            .cloned()
            .collect();
        representations.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(representations)
    }

    /// Returns clauses explicitly belonging to a contract in stable clause identity order.
    pub fn clauses_for(&self, contract: &ContractId) -> Result<Vec<ContractClause>, QueryError> {
        self.require_contract(contract)?;
        let mut clauses: Vec<_> = self
            .graph
            .clauses
            .iter()
            .filter(|clause| &clause.contract == contract)
            .cloned()
            .collect();
        clauses.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(clauses)
    }

    /// Returns source observations explicitly cited by interpretations attached to a contract.
    ///
    /// The closure includes clause assertions for the contract, relations touching the contract or
    /// one of its post-alignment representations, facet authority claims, and alignment claims when
    /// an alignment catalog was supplied. Unlinked observations are deliberately excluded.
    pub fn evidence_for(&self, contract: &ContractId) -> Result<Vec<Observation>, QueryError> {
        self.require_contract(contract)?;

        let representation_ids: BTreeSet<_> = self
            .graph
            .representations
            .iter()
            .filter(|representation| &representation.contract == contract)
            .map(|representation| representation.id.clone())
            .collect();
        let clause_ids: BTreeSet<_> = self
            .graph
            .clauses
            .iter()
            .filter(|clause| &clause.contract == contract)
            .map(|clause| clause.id.clone())
            .collect();

        let mut observation_ids = BTreeSet::new();
        for assertion in &self.graph.clause_assertions {
            if clause_ids.contains(&assertion.clause) {
                observation_ids.extend(assertion.evidence.iter().cloned());
            }
        }
        for relation in &self.graph.relations {
            if node_touches_contract(&relation.from, contract, &representation_ids)
                || node_touches_contract(&relation.to, contract, &representation_ids)
            {
                observation_ids.extend(relation.basis.iter().cloned());
            }
        }
        for claim in &self.graph.authority_claims {
            if &claim.contract == contract {
                observation_ids.extend(claim.evidence.iter().cloned());
            }
        }
        if let Some(alignments) = self.alignments {
            for claim in &alignments.claims {
                if &claim.contract == contract {
                    observation_ids.extend(claim.evidence.iter().cloned());
                }
            }
        }

        Ok(observations_by_id(self.graph, observation_ids))
    }

    /// Returns every currently contested clause without selecting a winner.
    #[must_use]
    pub fn contestations(&self) -> Vec<ClauseAssessment> {
        let mut clause_ids: Vec<_> = self
            .graph
            .clauses
            .iter()
            .map(|clause| clause.id.clone())
            .collect();
        clause_ids.sort();

        clause_ids
            .into_iter()
            .map(|clause| {
                self.graph
                    .assess_clause(&clause)
                    .expect("validated graph must assess a known clause")
            })
            .filter(|assessment| assessment.status == ClauseStatus::Contested)
            .collect()
    }

    /// Returns all authority claims for one contract facet, without ranking or resolving them.
    pub fn authority_for(
        &self,
        contract: &ContractId,
        facet: ContractFacet,
    ) -> Result<Vec<AuthorityClaim>, QueryError> {
        self.require_contract(contract)?;
        let mut claims: Vec<_> = self
            .graph
            .authority_claims
            .iter()
            .filter(|claim| &claim.contract == contract && claim.facet == facet)
            .cloned()
            .collect();
        claims.sort_by(|left, right| {
            (
                &left.representation,
                authority_basis_rank(left.basis),
                &left.evidence,
            )
                .cmp(&(
                    &right.representation,
                    authority_basis_rank(right.basis),
                    &right.evidence,
                ))
        });
        Ok(claims)
    }

    /// Returns only alignment claims explicitly recorded for an observed representation.
    #[must_use]
    pub fn alignments_for(&self, representation: &RepresentationId) -> Vec<AlignmentClaim> {
        self.alignments
            .map_or_else(Vec::new, |catalog| catalog.claims_for(representation))
    }

    fn require_contract(&self, contract: &ContractId) -> Result<(), QueryError> {
        if self
            .graph
            .contracts
            .iter()
            .any(|candidate| &candidate.id == contract)
        {
            Ok(())
        } else {
            Err(QueryError::UnknownContract(contract.clone()))
        }
    }
}

fn node_touches_contract(
    node: &NodeRef,
    contract: &ContractId,
    representations: &BTreeSet<RepresentationId>,
) -> bool {
    match node {
        NodeRef::Contract(candidate) => candidate == contract,
        NodeRef::Representation(candidate) => representations.contains(candidate),
    }
}

fn observations_by_id(
    graph: &ContractGraph,
    observation_ids: BTreeSet<ObservationId>,
) -> Vec<Observation> {
    observation_ids
        .into_iter()
        .filter_map(|id| {
            graph
                .observations
                .iter()
                .find(|observation| observation.id == id)
                .cloned()
        })
        .collect()
}

const fn authority_basis_rank(basis: AuthorityBasis) -> u8 {
    match basis {
        AuthorityBasis::ExplicitDeclaration => 0,
        AuthorityBasis::MechanicalEnforcement => 1,
        AuthorityBasis::ObservedBehavior => 2,
        AuthorityBasis::Documentation => 3,
        AuthorityBasis::Inference => 4,
    }
}
