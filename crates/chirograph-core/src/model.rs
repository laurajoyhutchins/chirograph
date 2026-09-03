use std::collections::BTreeSet;

macro_rules! id_type {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, IdentifierError> {
                let value = value.into();
                if value.is_empty() {
                    return Err(IdentifierError::Empty);
                }
                if value.trim() != value {
                    return Err(IdentifierError::SurroundingWhitespace);
                }
                Ok(Self(value))
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentifierError {
    Empty,
    SurroundingWhitespace,
}

id_type!(ContractId);
id_type!(RepresentationId);
id_type!(SourceId);
id_type!(ObservationId);
id_type!(ClauseId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContractFacet {
    Structural,
    Executable,
    Semantic,
    Failure,
    Concurrency,
    Recovery,
    Verification,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    Repository,
    FileSystem,
    Executable,
    Api,
    Url,
    Environment,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentationKind {
    ExecutableSurface,
    SourceCode,
    Schema,
    TypeDefinition,
    Validator,
    Test,
    Documentation,
    Configuration,
    GeneratedArtifact,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Revision {
    Exact(String),
    Unversioned,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub id: SourceId,
    pub kind: SourceKind,
    pub locator: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contract {
    pub id: ContractId,
    pub name: String,
    pub facets: Vec<ContractFacet>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Representation {
    pub id: RepresentationId,
    pub contract: ContractId,
    pub source: SourceId,
    pub kind: RepresentationKind,
    pub locator: String,
    pub facets: Vec<ContractFacet>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    pub id: ObservationId,
    pub source: SourceId,
    pub revision: Revision,
    pub locator: String,
    pub fact: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NodeRef {
    Contract(ContractId),
    Representation(RepresentationId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationKind {
    Defines,
    Implements,
    Documents,
    Validates,
    Generates,
    Projects,
    EquivalentTo,
    ConflictsWith,
    DependsOn,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relation {
    pub from: NodeRef,
    pub to: NodeRef,
    pub kind: RelationKind,
    pub basis: Vec<ObservationId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorityBasis {
    ExplicitDeclaration,
    MechanicalEnforcement,
    ObservedBehavior,
    Documentation,
    Inference,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityClaim {
    pub contract: ContractId,
    pub representation: RepresentationId,
    pub facet: ContractFacet,
    pub basis: AuthorityBasis,
    pub evidence: Vec<ObservationId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClauseKind {
    Requirement,
    Guarantee,
    Invariant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractClause {
    pub id: ClauseId,
    pub contract: ContractId,
    pub facet: ContractFacet,
    pub kind: ClauseKind,
    pub statement: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClauseStance {
    Supports,
    Contradicts,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClauseAssertion {
    pub clause: ClauseId,
    pub representation: RepresentationId,
    pub stance: ClauseStance,
    pub evidence: Vec<ObservationId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClauseStatus {
    Consistent,
    Contested,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClauseAssessment {
    pub clause: ClauseId,
    pub status: ClauseStatus,
    pub supporting_representations: Vec<RepresentationId>,
    pub contradicting_representations: Vec<RepresentationId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ContractGraph {
    pub sources: Vec<Source>,
    pub contracts: Vec<Contract>,
    pub representations: Vec<Representation>,
    pub observations: Vec<Observation>,
    pub clauses: Vec<ContractClause>,
    pub clause_assertions: Vec<ClauseAssertion>,
    pub relations: Vec<Relation>,
    pub authority_claims: Vec<AuthorityClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelError {
    DuplicateSource(SourceId),
    DuplicateContract(ContractId),
    DuplicateRepresentation(RepresentationId),
    DuplicateObservation(ObservationId),
    DuplicateClause(ClauseId),
    UnknownSource(SourceId),
    UnknownContract(ContractId),
    UnknownRepresentation(RepresentationId),
    UnknownObservation(ObservationId),
    UnknownClause(ClauseId),
    InvalidExactRevision(ObservationId),
    InvalidClauseStatement(ClauseId),
    ClauseFacetOutsideContract {
        clause: ClauseId,
        contract: ContractId,
        facet: ContractFacet,
    },
    ClauseAssertionContractMismatch {
        clause: ClauseId,
        representation: RepresentationId,
    },
    ClauseFacetNotRepresented {
        clause: ClauseId,
        representation: RepresentationId,
        facet: ContractFacet,
    },
    ClauseAssertionEvidenceRequired {
        clause: ClauseId,
        representation: RepresentationId,
    },
    ClauseSupportRequired(ClauseId),
    RepresentationFacetOutsideContract {
        representation: RepresentationId,
        contract: ContractId,
        facet: ContractFacet,
    },
    AuthorityFacetNotRepresented {
        representation: RepresentationId,
        facet: ContractFacet,
    },
    RelationEvidenceRequired {
        from: NodeRef,
        to: NodeRef,
        kind: RelationKind,
    },
    AuthorityEvidenceRequired {
        contract: ContractId,
        representation: RepresentationId,
    },
    AuthorityContractMismatch {
        contract: ContractId,
        representation: RepresentationId,
    },
}

impl ContractGraph {
    pub fn validate(&self) -> Result<(), ModelError> {
        let source_ids = unique_ids(
            self.sources.iter().map(|source| source.id.clone()),
            ModelError::DuplicateSource,
        )?;
        let contract_ids = unique_ids(
            self.contracts.iter().map(|contract| contract.id.clone()),
            ModelError::DuplicateContract,
        )?;
        let representation_ids = unique_ids(
            self.representations
                .iter()
                .map(|representation| representation.id.clone()),
            ModelError::DuplicateRepresentation,
        )?;
        let observation_ids = unique_ids(
            self.observations
                .iter()
                .map(|observation| observation.id.clone()),
            ModelError::DuplicateObservation,
        )?;
        let clause_ids = unique_ids(
            self.clauses.iter().map(|clause| clause.id.clone()),
            ModelError::DuplicateClause,
        )?;

        for representation in &self.representations {
            if !source_ids.contains(&representation.source) {
                return Err(ModelError::UnknownSource(representation.source.clone()));
            }
            if !contract_ids.contains(&representation.contract) {
                return Err(ModelError::UnknownContract(representation.contract.clone()));
            }
            let contract = self
                .contracts
                .iter()
                .find(|contract| contract.id == representation.contract)
                .expect("known contract id must resolve");
            for facet in &representation.facets {
                if !contract.facets.contains(facet) {
                    return Err(ModelError::RepresentationFacetOutsideContract {
                        representation: representation.id.clone(),
                        contract: representation.contract.clone(),
                        facet: *facet,
                    });
                }
            }
        }

        for observation in &self.observations {
            if !source_ids.contains(&observation.source) {
                return Err(ModelError::UnknownSource(observation.source.clone()));
            }
            if let Revision::Exact(revision) = &observation.revision
                && (revision.is_empty() || revision.trim() != revision)
            {
                return Err(ModelError::InvalidExactRevision(observation.id.clone()));
            }
        }

        for clause in &self.clauses {
            if !contract_ids.contains(&clause.contract) {
                return Err(ModelError::UnknownContract(clause.contract.clone()));
            }
            if clause.statement.is_empty() || clause.statement.trim() != clause.statement {
                return Err(ModelError::InvalidClauseStatement(clause.id.clone()));
            }
            let contract = self
                .contracts
                .iter()
                .find(|contract| contract.id == clause.contract)
                .expect("known contract id must resolve");
            if !contract.facets.contains(&clause.facet) {
                return Err(ModelError::ClauseFacetOutsideContract {
                    clause: clause.id.clone(),
                    contract: clause.contract.clone(),
                    facet: clause.facet,
                });
            }
        }

        for assertion in &self.clause_assertions {
            if !clause_ids.contains(&assertion.clause) {
                return Err(ModelError::UnknownClause(assertion.clause.clone()));
            }
            if !representation_ids.contains(&assertion.representation) {
                return Err(ModelError::UnknownRepresentation(
                    assertion.representation.clone(),
                ));
            }
            let clause = self
                .clauses
                .iter()
                .find(|clause| clause.id == assertion.clause)
                .expect("known clause id must resolve");
            let representation = self
                .representations
                .iter()
                .find(|representation| representation.id == assertion.representation)
                .expect("known representation id must resolve");
            if representation.contract != clause.contract {
                return Err(ModelError::ClauseAssertionContractMismatch {
                    clause: clause.id.clone(),
                    representation: representation.id.clone(),
                });
            }
            if !representation.facets.contains(&clause.facet) {
                return Err(ModelError::ClauseFacetNotRepresented {
                    clause: clause.id.clone(),
                    representation: representation.id.clone(),
                    facet: clause.facet,
                });
            }
            if assertion.evidence.is_empty() {
                return Err(ModelError::ClauseAssertionEvidenceRequired {
                    clause: clause.id.clone(),
                    representation: representation.id.clone(),
                });
            }
            validate_observations(&assertion.evidence, &observation_ids)?;
        }

        for clause in &self.clauses {
            let supported = self.clause_assertions.iter().any(|assertion| {
                assertion.clause == clause.id && assertion.stance == ClauseStance::Supports
            });
            if !supported {
                return Err(ModelError::ClauseSupportRequired(clause.id.clone()));
            }
        }

        for relation in &self.relations {
            validate_node(&relation.from, &contract_ids, &representation_ids)?;
            validate_node(&relation.to, &contract_ids, &representation_ids)?;
            if relation.basis.is_empty() {
                return Err(ModelError::RelationEvidenceRequired {
                    from: relation.from.clone(),
                    to: relation.to.clone(),
                    kind: relation.kind,
                });
            }
            validate_observations(&relation.basis, &observation_ids)?;
        }

        for claim in &self.authority_claims {
            if !contract_ids.contains(&claim.contract) {
                return Err(ModelError::UnknownContract(claim.contract.clone()));
            }
            let Some(representation) = self
                .representations
                .iter()
                .find(|representation| representation.id == claim.representation)
            else {
                return Err(ModelError::UnknownRepresentation(
                    claim.representation.clone(),
                ));
            };
            if representation.contract != claim.contract {
                return Err(ModelError::AuthorityContractMismatch {
                    contract: claim.contract.clone(),
                    representation: claim.representation.clone(),
                });
            }
            if !representation.facets.contains(&claim.facet) {
                return Err(ModelError::AuthorityFacetNotRepresented {
                    representation: claim.representation.clone(),
                    facet: claim.facet,
                });
            }
            if claim.evidence.is_empty() {
                return Err(ModelError::AuthorityEvidenceRequired {
                    contract: claim.contract.clone(),
                    representation: claim.representation.clone(),
                });
            }
            validate_observations(&claim.evidence, &observation_ids)?;
        }

        Ok(())
    }

    pub fn assess_clause(&self, clause_id: &ClauseId) -> Result<ClauseAssessment, ModelError> {
        self.validate()?;
        if !self.clauses.iter().any(|clause| &clause.id == clause_id) {
            return Err(ModelError::UnknownClause(clause_id.clone()));
        }

        let mut supporting = BTreeSet::new();
        let mut contradicting = BTreeSet::new();
        for assertion in self
            .clause_assertions
            .iter()
            .filter(|assertion| &assertion.clause == clause_id)
        {
            match assertion.stance {
                ClauseStance::Supports => {
                    supporting.insert(assertion.representation.clone());
                }
                ClauseStance::Contradicts => {
                    contradicting.insert(assertion.representation.clone());
                }
            }
        }

        let status = if contradicting.is_empty() {
            ClauseStatus::Consistent
        } else {
            ClauseStatus::Contested
        };
        Ok(ClauseAssessment {
            clause: clause_id.clone(),
            status,
            supporting_representations: supporting.into_iter().collect(),
            contradicting_representations: contradicting.into_iter().collect(),
        })
    }
}

fn unique_ids<T, F>(ids: impl Iterator<Item = T>, duplicate: F) -> Result<BTreeSet<T>, ModelError>
where
    T: Ord + Clone,
    F: Fn(T) -> ModelError,
{
    let mut seen = BTreeSet::new();
    for id in ids {
        if !seen.insert(id.clone()) {
            return Err(duplicate(id));
        }
    }
    Ok(seen)
}

fn validate_node(
    node: &NodeRef,
    contracts: &BTreeSet<ContractId>,
    representations: &BTreeSet<RepresentationId>,
) -> Result<(), ModelError> {
    match node {
        NodeRef::Contract(id) if !contracts.contains(id) => {
            Err(ModelError::UnknownContract(id.clone()))
        }
        NodeRef::Representation(id) if !representations.contains(id) => {
            Err(ModelError::UnknownRepresentation(id.clone()))
        }
        _ => Ok(()),
    }
}

fn validate_observations(
    ids: &[ObservationId],
    observations: &BTreeSet<ObservationId>,
) -> Result<(), ModelError> {
    for id in ids {
        if !observations.contains(id) {
            return Err(ModelError::UnknownObservation(id.clone()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract_id(value: &str) -> ContractId {
        ContractId::new(value).expect("valid contract id")
    }

    fn representation_id(value: &str) -> RepresentationId {
        RepresentationId::new(value).expect("valid representation id")
    }

    fn source_id(value: &str) -> SourceId {
        SourceId::new(value).expect("valid source id")
    }

    fn observation_id(value: &str) -> ObservationId {
        ObservationId::new(value).expect("valid observation id")
    }

    fn graph() -> ContractGraph {
        let contract = contract_id("git.update-ref");
        let source = source_id("git-help");
        let representation = representation_id("git-update-ref-help");
        let observation = observation_id("obs-update-ref-old-oid");

        ContractGraph {
            sources: vec![Source {
                id: source.clone(),
                kind: SourceKind::Executable,
                locator: "/usr/bin/git".into(),
            }],
            contracts: vec![Contract {
                id: contract.clone(),
                name: "git update-ref".into(),
                facets: vec![ContractFacet::Concurrency, ContractFacet::Failure],
            }],
            representations: vec![Representation {
                id: representation.clone(),
                contract: contract.clone(),
                source: source.clone(),
                kind: RepresentationKind::ExecutableSurface,
                locator: "git update-ref --help".into(),
                facets: vec![ContractFacet::Concurrency],
            }],
            observations: vec![Observation {
                id: observation.clone(),
                source,
                revision: Revision::Exact("git 2.47.3".into()),
                locator: "update-ref help".into(),
                fact: "old object id acts as an expected-current-value precondition".into(),
            }],
            clauses: vec![],
            clause_assertions: vec![],
            relations: vec![Relation {
                from: NodeRef::Representation(representation.clone()),
                to: NodeRef::Contract(contract.clone()),
                kind: RelationKind::Documents,
                basis: vec![observation.clone()],
            }],
            authority_claims: vec![AuthorityClaim {
                contract,
                representation,
                facet: ContractFacet::Concurrency,
                basis: AuthorityBasis::Documentation,
                evidence: vec![observation],
            }],
        }
    }

    #[test]
    fn accepts_source_backed_contract_graph() {
        graph().validate().expect("graph should be valid");
    }

    #[test]
    fn observations_keep_exactness_explicit() {
        let graph = graph();
        assert_eq!(
            graph.observations[0].revision,
            Revision::Exact("git 2.47.3".into())
        );
    }

    #[test]
    fn rejects_representation_with_unknown_source() {
        let mut graph = graph();
        graph.representations[0].source = source_id("missing");
        assert_eq!(
            graph.validate(),
            Err(ModelError::UnknownSource(source_id("missing")))
        );
    }

    #[test]
    fn rejects_representation_facet_outside_contract() {
        let mut graph = graph();
        let representation = graph.representations[0].id.clone();
        let contract = graph.representations[0].contract.clone();
        graph.representations[0].facets = vec![ContractFacet::Verification];
        assert_eq!(
            graph.validate(),
            Err(ModelError::RepresentationFacetOutsideContract {
                representation,
                contract,
                facet: ContractFacet::Verification,
            })
        );
    }

    #[test]
    fn rejects_relation_without_observation_basis() {
        let mut graph = graph();
        let from = graph.relations[0].from.clone();
        let to = graph.relations[0].to.clone();
        graph.relations[0].basis.clear();
        assert_eq!(
            graph.validate(),
            Err(ModelError::RelationEvidenceRequired {
                from,
                to,
                kind: RelationKind::Documents,
            })
        );
    }

    #[test]
    fn rejects_relation_with_unknown_observation_basis() {
        let mut graph = graph();
        graph.relations[0].basis = vec![observation_id("missing")];
        assert_eq!(
            graph.validate(),
            Err(ModelError::UnknownObservation(observation_id("missing")))
        );
    }

    #[test]
    fn rejects_authority_claim_without_evidence() {
        let mut graph = graph();
        let contract = graph.authority_claims[0].contract.clone();
        let representation = graph.authority_claims[0].representation.clone();
        graph.authority_claims[0].evidence.clear();
        assert_eq!(
            graph.validate(),
            Err(ModelError::AuthorityEvidenceRequired {
                contract,
                representation,
            })
        );
    }

    #[test]
    fn rejects_authority_claim_for_unrepresented_facet() {
        let mut graph = graph();
        let representation = graph.authority_claims[0].representation.clone();
        graph.authority_claims[0].facet = ContractFacet::Failure;
        assert_eq!(
            graph.validate(),
            Err(ModelError::AuthorityFacetNotRepresented {
                representation,
                facet: ContractFacet::Failure,
            })
        );
    }

    #[test]
    fn rejects_authority_claim_for_another_contract() {
        let mut graph = graph();
        let other = contract_id("other-contract");
        graph.contracts.push(Contract {
            id: other.clone(),
            name: "other".into(),
            facets: vec![ContractFacet::Semantic],
        });
        graph.authority_claims[0].contract = other.clone();
        assert_eq!(
            graph.validate(),
            Err(ModelError::AuthorityContractMismatch {
                contract: other,
                representation: representation_id("git-update-ref-help"),
            })
        );
    }

    #[test]
    fn rejects_empty_exact_revision() {
        let mut graph = graph();
        let observation = graph.observations[0].id.clone();
        graph.observations[0].revision = Revision::Exact(String::new());
        assert_eq!(
            graph.validate(),
            Err(ModelError::InvalidExactRevision(observation))
        );
    }

    #[test]
    fn identifiers_do_not_silently_normalize() {
        assert_eq!(
            ContractId::new(" contract "),
            Err(IdentifierError::SurroundingWhitespace)
        );
    }
}
