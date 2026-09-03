//! Versioned, language-neutral evidence interchange for Chirograph.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::model::{
    AuthorityBasis, AuthorityClaim, ClauseAssertion, ClauseId, ClauseKind, ClauseStance, Contract,
    ContractClause, ContractFacet, ContractGraph, ContractId, ModelError, NodeRef, Observation,
    ObservationId, Relation, RelationKind, Representation, RepresentationId, RepresentationKind,
    Revision, Source, SourceId, SourceKind,
};

pub const EVIDENCE_SCHEMA_V1: &str = "chirograph-evidence-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceError {
    InvalidJson(String),
    UnsupportedSchema(String),
    InvalidIdentifier {
        kind: String,
        value: String,
        reason: String,
    },
    InvalidValue {
        field: String,
        value: String,
    },
    InvalidGraph(ModelError),
    Serialization(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvidenceDocumentV1 {
    schema: String,
    sources: Vec<SourceWire>,
    contracts: Vec<ContractWire>,
    representations: Vec<RepresentationWire>,
    observations: Vec<ObservationWire>,
    clauses: Vec<ContractClauseWire>,
    clause_assertions: Vec<ClauseAssertionWire>,
    relations: Vec<RelationWire>,
    authority_claims: Vec<AuthorityClaimWire>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceWire {
    id: String,
    kind: String,
    locator: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContractWire {
    id: String,
    name: String,
    facets: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RepresentationWire {
    id: String,
    contract: String,
    source: String,
    kind: String,
    locator: String,
    facets: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ObservationWire {
    id: String,
    source: String,
    revision: RevisionWire,
    locator: String,
    fact: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum RevisionWire {
    Exact { value: String },
    Unversioned,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContractClauseWire {
    id: String,
    contract: String,
    facet: String,
    kind: String,
    statement: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClauseAssertionWire {
    clause: String,
    representation: String,
    stance: String,
    evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RelationWire {
    from: NodeRefWire,
    to: NodeRefWire,
    kind: String,
    basis: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum NodeRefWire {
    Contract { id: String },
    Representation { id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthorityClaimWire {
    contract: String,
    representation: String,
    facet: String,
    basis: String,
    evidence: Vec<String>,
}

pub fn parse_evidence_json(input: &str) -> Result<ContractGraph, EvidenceError> {
    let value: Value = serde_json::from_str(input)
        .map_err(|error| EvidenceError::InvalidJson(error.to_string()))?;
    let schema = value
        .get("schema")
        .and_then(Value::as_str)
        .ok_or_else(|| EvidenceError::InvalidJson("schema must be a string".into()))?;
    if schema != EVIDENCE_SCHEMA_V1 {
        return Err(EvidenceError::UnsupportedSchema(schema.into()));
    }

    let document: EvidenceDocumentV1 = serde_json::from_value(value)
        .map_err(|error| EvidenceError::InvalidJson(error.to_string()))?;
    let graph = document.into_graph()?;
    graph.validate().map_err(EvidenceError::InvalidGraph)?;
    Ok(graph)
}

pub fn render_evidence_json_pretty(graph: &ContractGraph) -> Result<String, EvidenceError> {
    graph.validate().map_err(EvidenceError::InvalidGraph)?;
    serde_json::to_string_pretty(&EvidenceDocumentV1::from_graph(graph))
        .map_err(|error| EvidenceError::Serialization(error.to_string()))
}

impl EvidenceDocumentV1 {
    fn into_graph(self) -> Result<ContractGraph, EvidenceError> {
        Ok(ContractGraph {
            sources: self
                .sources
                .into_iter()
                .map(|source| {
                    Ok(Source {
                        id: source_id(source.id)?,
                        kind: parse_source_kind(&source.kind)?,
                        locator: source.locator,
                    })
                })
                .collect::<Result<Vec<_>, EvidenceError>>()?,
            contracts: self
                .contracts
                .into_iter()
                .map(|contract| {
                    Ok(Contract {
                        id: contract_id(contract.id)?,
                        name: contract.name,
                        facets: contract
                            .facets
                            .into_iter()
                            .map(|facet| parse_facet(&facet))
                            .collect::<Result<Vec<_>, EvidenceError>>()?,
                    })
                })
                .collect::<Result<Vec<_>, EvidenceError>>()?,
            representations: self
                .representations
                .into_iter()
                .map(|representation| {
                    Ok(Representation {
                        id: representation_id(representation.id)?,
                        contract: contract_id(representation.contract)?,
                        source: source_id(representation.source)?,
                        kind: parse_representation_kind(&representation.kind)?,
                        locator: representation.locator,
                        facets: representation
                            .facets
                            .into_iter()
                            .map(|facet| parse_facet(&facet))
                            .collect::<Result<Vec<_>, EvidenceError>>()?,
                    })
                })
                .collect::<Result<Vec<_>, EvidenceError>>()?,
            observations: self
                .observations
                .into_iter()
                .map(|observation| {
                    Ok(Observation {
                        id: observation_id(observation.id)?,
                        source: source_id(observation.source)?,
                        revision: match observation.revision {
                            RevisionWire::Exact { value } => Revision::Exact(value),
                            RevisionWire::Unversioned => Revision::Unversioned,
                            RevisionWire::Unknown => Revision::Unknown,
                        },
                        locator: observation.locator,
                        fact: observation.fact,
                    })
                })
                .collect::<Result<Vec<_>, EvidenceError>>()?,
            clauses: self
                .clauses
                .into_iter()
                .map(|clause| {
                    Ok(ContractClause {
                        id: clause_id(clause.id)?,
                        contract: contract_id(clause.contract)?,
                        facet: parse_facet(&clause.facet)?,
                        kind: parse_clause_kind(&clause.kind)?,
                        statement: clause.statement,
                    })
                })
                .collect::<Result<Vec<_>, EvidenceError>>()?,
            clause_assertions: self
                .clause_assertions
                .into_iter()
                .map(|assertion| {
                    Ok(ClauseAssertion {
                        clause: clause_id(assertion.clause)?,
                        representation: representation_id(assertion.representation)?,
                        stance: parse_clause_stance(&assertion.stance)?,
                        evidence: assertion
                            .evidence
                            .into_iter()
                            .map(observation_id)
                            .collect::<Result<Vec<_>, EvidenceError>>()?,
                    })
                })
                .collect::<Result<Vec<_>, EvidenceError>>()?,
            relations: self
                .relations
                .into_iter()
                .map(|relation| {
                    Ok(Relation {
                        from: parse_node_ref(relation.from)?,
                        to: parse_node_ref(relation.to)?,
                        kind: parse_relation_kind(&relation.kind)?,
                        basis: relation
                            .basis
                            .into_iter()
                            .map(observation_id)
                            .collect::<Result<Vec<_>, EvidenceError>>()?,
                    })
                })
                .collect::<Result<Vec<_>, EvidenceError>>()?,
            authority_claims: self
                .authority_claims
                .into_iter()
                .map(|claim| {
                    Ok(AuthorityClaim {
                        contract: contract_id(claim.contract)?,
                        representation: representation_id(claim.representation)?,
                        facet: parse_facet(&claim.facet)?,
                        basis: parse_authority_basis(&claim.basis)?,
                        evidence: claim
                            .evidence
                            .into_iter()
                            .map(observation_id)
                            .collect::<Result<Vec<_>, EvidenceError>>()?,
                    })
                })
                .collect::<Result<Vec<_>, EvidenceError>>()?,
        })
    }

    fn from_graph(graph: &ContractGraph) -> Self {
        Self {
            schema: EVIDENCE_SCHEMA_V1.into(),
            sources: graph
                .sources
                .iter()
                .map(|source| SourceWire {
                    id: source.id.as_str().into(),
                    kind: source_kind_name(source.kind).into(),
                    locator: source.locator.clone(),
                })
                .collect(),
            contracts: graph
                .contracts
                .iter()
                .map(|contract| ContractWire {
                    id: contract.id.as_str().into(),
                    name: contract.name.clone(),
                    facets: contract
                        .facets
                        .iter()
                        .map(|facet| facet_name(*facet).into())
                        .collect(),
                })
                .collect(),
            representations: graph
                .representations
                .iter()
                .map(|representation| RepresentationWire {
                    id: representation.id.as_str().into(),
                    contract: representation.contract.as_str().into(),
                    source: representation.source.as_str().into(),
                    kind: representation_kind_name(representation.kind).into(),
                    locator: representation.locator.clone(),
                    facets: representation
                        .facets
                        .iter()
                        .map(|facet| facet_name(*facet).into())
                        .collect(),
                })
                .collect(),
            observations: graph
                .observations
                .iter()
                .map(|observation| ObservationWire {
                    id: observation.id.as_str().into(),
                    source: observation.source.as_str().into(),
                    revision: match &observation.revision {
                        Revision::Exact(value) => RevisionWire::Exact {
                            value: value.clone(),
                        },
                        Revision::Unversioned => RevisionWire::Unversioned,
                        Revision::Unknown => RevisionWire::Unknown,
                    },
                    locator: observation.locator.clone(),
                    fact: observation.fact.clone(),
                })
                .collect(),
            clauses: graph
                .clauses
                .iter()
                .map(|clause| ContractClauseWire {
                    id: clause.id.as_str().into(),
                    contract: clause.contract.as_str().into(),
                    facet: facet_name(clause.facet).into(),
                    kind: clause_kind_name(clause.kind).into(),
                    statement: clause.statement.clone(),
                })
                .collect(),
            clause_assertions: graph
                .clause_assertions
                .iter()
                .map(|assertion| ClauseAssertionWire {
                    clause: assertion.clause.as_str().into(),
                    representation: assertion.representation.as_str().into(),
                    stance: clause_stance_name(assertion.stance).into(),
                    evidence: assertion
                        .evidence
                        .iter()
                        .map(|id| id.as_str().into())
                        .collect(),
                })
                .collect(),
            relations: graph
                .relations
                .iter()
                .map(|relation| RelationWire {
                    from: node_ref_wire(&relation.from),
                    to: node_ref_wire(&relation.to),
                    kind: relation_kind_name(relation.kind).into(),
                    basis: relation.basis.iter().map(|id| id.as_str().into()).collect(),
                })
                .collect(),
            authority_claims: graph
                .authority_claims
                .iter()
                .map(|claim| AuthorityClaimWire {
                    contract: claim.contract.as_str().into(),
                    representation: claim.representation.as_str().into(),
                    facet: facet_name(claim.facet).into(),
                    basis: authority_basis_name(claim.basis).into(),
                    evidence: claim.evidence.iter().map(|id| id.as_str().into()).collect(),
                })
                .collect(),
        }
    }
}

fn identifier_error(kind: &str, value: String, error: impl std::fmt::Debug) -> EvidenceError {
    EvidenceError::InvalidIdentifier {
        kind: kind.into(),
        value,
        reason: format!("{error:?}"),
    }
}

fn contract_id(value: String) -> Result<ContractId, EvidenceError> {
    ContractId::new(value.clone()).map_err(|error| identifier_error("contract", value, error))
}

fn representation_id(value: String) -> Result<RepresentationId, EvidenceError> {
    RepresentationId::new(value.clone())
        .map_err(|error| identifier_error("representation", value, error))
}

fn source_id(value: String) -> Result<SourceId, EvidenceError> {
    SourceId::new(value.clone()).map_err(|error| identifier_error("source", value, error))
}

fn observation_id(value: String) -> Result<ObservationId, EvidenceError> {
    ObservationId::new(value.clone()).map_err(|error| identifier_error("observation", value, error))
}

fn clause_id(value: String) -> Result<ClauseId, EvidenceError> {
    ClauseId::new(value.clone()).map_err(|error| identifier_error("clause", value, error))
}

fn invalid_value(field: &str, value: &str) -> EvidenceError {
    EvidenceError::InvalidValue {
        field: field.into(),
        value: value.into(),
    }
}

fn parse_facet(value: &str) -> Result<ContractFacet, EvidenceError> {
    match value {
        "structural" => Ok(ContractFacet::Structural),
        "executable" => Ok(ContractFacet::Executable),
        "semantic" => Ok(ContractFacet::Semantic),
        "failure" => Ok(ContractFacet::Failure),
        "concurrency" => Ok(ContractFacet::Concurrency),
        "recovery" => Ok(ContractFacet::Recovery),
        "verification" => Ok(ContractFacet::Verification),
        _ => Err(invalid_value("facet", value)),
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

fn parse_source_kind(value: &str) -> Result<SourceKind, EvidenceError> {
    match value {
        "repository" => Ok(SourceKind::Repository),
        "file_system" => Ok(SourceKind::FileSystem),
        "executable" => Ok(SourceKind::Executable),
        "api" => Ok(SourceKind::Api),
        "url" => Ok(SourceKind::Url),
        "environment" => Ok(SourceKind::Environment),
        "other" => Ok(SourceKind::Other),
        _ => Err(invalid_value("source.kind", value)),
    }
}

const fn source_kind_name(value: SourceKind) -> &'static str {
    match value {
        SourceKind::Repository => "repository",
        SourceKind::FileSystem => "file_system",
        SourceKind::Executable => "executable",
        SourceKind::Api => "api",
        SourceKind::Url => "url",
        SourceKind::Environment => "environment",
        SourceKind::Other => "other",
    }
}

fn parse_representation_kind(value: &str) -> Result<RepresentationKind, EvidenceError> {
    match value {
        "executable_surface" => Ok(RepresentationKind::ExecutableSurface),
        "source_code" => Ok(RepresentationKind::SourceCode),
        "schema" => Ok(RepresentationKind::Schema),
        "type_definition" => Ok(RepresentationKind::TypeDefinition),
        "validator" => Ok(RepresentationKind::Validator),
        "test" => Ok(RepresentationKind::Test),
        "documentation" => Ok(RepresentationKind::Documentation),
        "configuration" => Ok(RepresentationKind::Configuration),
        "generated_artifact" => Ok(RepresentationKind::GeneratedArtifact),
        "other" => Ok(RepresentationKind::Other),
        _ => Err(invalid_value("representation.kind", value)),
    }
}

const fn representation_kind_name(value: RepresentationKind) -> &'static str {
    match value {
        RepresentationKind::ExecutableSurface => "executable_surface",
        RepresentationKind::SourceCode => "source_code",
        RepresentationKind::Schema => "schema",
        RepresentationKind::TypeDefinition => "type_definition",
        RepresentationKind::Validator => "validator",
        RepresentationKind::Test => "test",
        RepresentationKind::Documentation => "documentation",
        RepresentationKind::Configuration => "configuration",
        RepresentationKind::GeneratedArtifact => "generated_artifact",
        RepresentationKind::Other => "other",
    }
}

fn parse_clause_kind(value: &str) -> Result<ClauseKind, EvidenceError> {
    match value {
        "requirement" => Ok(ClauseKind::Requirement),
        "guarantee" => Ok(ClauseKind::Guarantee),
        "invariant" => Ok(ClauseKind::Invariant),
        _ => Err(invalid_value("clause.kind", value)),
    }
}

const fn clause_kind_name(value: ClauseKind) -> &'static str {
    match value {
        ClauseKind::Requirement => "requirement",
        ClauseKind::Guarantee => "guarantee",
        ClauseKind::Invariant => "invariant",
    }
}

fn parse_clause_stance(value: &str) -> Result<ClauseStance, EvidenceError> {
    match value {
        "supports" => Ok(ClauseStance::Supports),
        "contradicts" => Ok(ClauseStance::Contradicts),
        _ => Err(invalid_value("clause_assertion.stance", value)),
    }
}

const fn clause_stance_name(value: ClauseStance) -> &'static str {
    match value {
        ClauseStance::Supports => "supports",
        ClauseStance::Contradicts => "contradicts",
    }
}

fn parse_relation_kind(value: &str) -> Result<RelationKind, EvidenceError> {
    match value {
        "defines" => Ok(RelationKind::Defines),
        "implements" => Ok(RelationKind::Implements),
        "documents" => Ok(RelationKind::Documents),
        "validates" => Ok(RelationKind::Validates),
        "generates" => Ok(RelationKind::Generates),
        "projects" => Ok(RelationKind::Projects),
        "equivalent_to" => Ok(RelationKind::EquivalentTo),
        "conflicts_with" => Ok(RelationKind::ConflictsWith),
        "depends_on" => Ok(RelationKind::DependsOn),
        _ => Err(invalid_value("relation.kind", value)),
    }
}

const fn relation_kind_name(value: RelationKind) -> &'static str {
    match value {
        RelationKind::Defines => "defines",
        RelationKind::Implements => "implements",
        RelationKind::Documents => "documents",
        RelationKind::Validates => "validates",
        RelationKind::Generates => "generates",
        RelationKind::Projects => "projects",
        RelationKind::EquivalentTo => "equivalent_to",
        RelationKind::ConflictsWith => "conflicts_with",
        RelationKind::DependsOn => "depends_on",
    }
}

fn parse_authority_basis(value: &str) -> Result<AuthorityBasis, EvidenceError> {
    match value {
        "explicit_declaration" => Ok(AuthorityBasis::ExplicitDeclaration),
        "mechanical_enforcement" => Ok(AuthorityBasis::MechanicalEnforcement),
        "observed_behavior" => Ok(AuthorityBasis::ObservedBehavior),
        "documentation" => Ok(AuthorityBasis::Documentation),
        "inference" => Ok(AuthorityBasis::Inference),
        _ => Err(invalid_value("authority_claim.basis", value)),
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

fn parse_node_ref(value: NodeRefWire) -> Result<NodeRef, EvidenceError> {
    match value {
        NodeRefWire::Contract { id } => Ok(NodeRef::Contract(contract_id(id)?)),
        NodeRefWire::Representation { id } => Ok(NodeRef::Representation(representation_id(id)?)),
    }
}

fn node_ref_wire(value: &NodeRef) -> NodeRefWire {
    match value {
        NodeRef::Contract(id) => NodeRefWire::Contract {
            id: id.as_str().into(),
        },
        NodeRef::Representation(id) => NodeRefWire::Representation {
            id: id.as_str().into(),
        },
    }
}
