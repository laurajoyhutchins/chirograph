use serde::{Deserialize, Serialize};

use crate::model::{
    AuthorityBasis, ClauseKind, ClauseStatus, ContractFacet, ContractGraph, ModelError, NodeRef,
    RelationKind, RepresentationKind,
};

pub const GRAPH_JSON_SCHEMA: &str = "chirograph-graph-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphJsonV1 {
    pub schema: String,
    pub contracts: Vec<GraphContractV1>,
    pub representations: Vec<GraphRepresentationV1>,
    pub relations: Vec<GraphRelationV1>,
    pub authority_claims: Vec<GraphAuthorityClaimV1>,
    pub clauses: Vec<GraphClauseV1>,
    pub clause_assessments: Vec<GraphClauseAssessmentV1>,
    #[serde(default)]
    pub lifecycle: Vec<GraphLifecycleV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphContractV1 {
    pub id: String,
    pub name: String,
    pub facets: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphRepresentationV1 {
    pub id: String,
    pub contract: String,
    pub kind: String,
    pub locator: String,
    pub facets: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphNodeRefV1 {
    pub kind: String,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphRelationV1 {
    pub from: GraphNodeRefV1,
    pub kind: String,
    pub to: GraphNodeRefV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphAuthorityClaimV1 {
    pub contract: String,
    pub representation: String,
    pub facet: String,
    pub basis: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphClauseV1 {
    pub id: String,
    pub contract: String,
    pub facet: String,
    pub kind: String,
    pub statement: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphClauseAssessmentV1 {
    pub clause: String,
    pub status: String,
    pub supporting_representations: Vec<String>,
    pub contradicting_representations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphLifecycleV1 {
    pub subject: GraphNodeRefV1,
    pub status: String,
}

#[derive(Debug)]
pub enum GraphJsonError {
    InvalidGraph(ModelError),
    Encode(serde_json::Error),
}

impl From<ModelError> for GraphJsonError {
    fn from(value: ModelError) -> Self {
        Self::InvalidGraph(value)
    }
}

impl From<serde_json::Error> for GraphJsonError {
    fn from(value: serde_json::Error) -> Self {
        Self::Encode(value)
    }
}

pub fn encode_graph_json(graph: &ContractGraph) -> Result<String, GraphJsonError> {
    graph.validate()?;

    let mut contracts = graph
        .contracts
        .iter()
        .map(|contract| GraphContractV1 {
            id: contract.id.as_str().to_owned(),
            name: contract.name.clone(),
            facets: facets(&contract.facets),
        })
        .collect::<Vec<_>>();
    contracts.sort_by(|left, right| left.id.cmp(&right.id));

    let mut representations = graph
        .representations
        .iter()
        .map(|representation| GraphRepresentationV1 {
            id: representation.id.as_str().to_owned(),
            contract: representation.contract.as_str().to_owned(),
            kind: representation_kind(representation.kind).to_owned(),
            locator: representation.locator.clone(),
            facets: facets(&representation.facets),
        })
        .collect::<Vec<_>>();
    representations.sort_by(|left, right| left.id.cmp(&right.id));

    let mut relations = graph
        .relations
        .iter()
        .map(|relation| GraphRelationV1 {
            from: node_ref(&relation.from),
            kind: relation_kind(relation.kind).to_owned(),
            to: node_ref(&relation.to),
        })
        .collect::<Vec<_>>();
    relations.sort_by(|left, right| {
        (&left.from, &left.kind, &left.to).cmp(&(&right.from, &right.kind, &right.to))
    });

    let mut authority_claims = graph
        .authority_claims
        .iter()
        .map(|claim| GraphAuthorityClaimV1 {
            contract: claim.contract.as_str().to_owned(),
            representation: claim.representation.as_str().to_owned(),
            facet: facet(claim.facet).to_owned(),
            basis: authority_basis(claim.basis).to_owned(),
        })
        .collect::<Vec<_>>();
    authority_claims.sort_by(|left, right| {
        (&left.contract, &left.facet, &left.representation).cmp(&(
            &right.contract,
            &right.facet,
            &right.representation,
        ))
    });

    let mut clauses = graph
        .clauses
        .iter()
        .map(|clause| GraphClauseV1 {
            id: clause.id.as_str().to_owned(),
            contract: clause.contract.as_str().to_owned(),
            facet: facet(clause.facet).to_owned(),
            kind: clause_kind(clause.kind).to_owned(),
            statement: clause.statement.clone(),
        })
        .collect::<Vec<_>>();
    clauses.sort_by(|left, right| left.id.cmp(&right.id));

    let mut clause_assessments = Vec::with_capacity(graph.clauses.len());
    for clause in &graph.clauses {
        let assessment = graph.assess_clause(&clause.id)?;
        clause_assessments.push(GraphClauseAssessmentV1 {
            clause: assessment.clause.as_str().to_owned(),
            status: clause_status(assessment.status).to_owned(),
            supporting_representations: assessment
                .supporting_representations
                .iter()
                .map(|id| id.as_str().to_owned())
                .collect(),
            contradicting_representations: assessment
                .contradicting_representations
                .iter()
                .map(|id| id.as_str().to_owned())
                .collect(),
        });
    }
    clause_assessments.sort_by(|left, right| left.clause.cmp(&right.clause));

    let output = GraphJsonV1 {
        schema: GRAPH_JSON_SCHEMA.to_owned(),
        contracts,
        representations,
        relations,
        authority_claims,
        clauses,
        clause_assessments,
        lifecycle: Vec::new(),
    };

    Ok(serde_json::to_string(&output)?)
}

fn facets(values: &[ContractFacet]) -> Vec<String> {
    let mut output = values
        .iter()
        .map(|value| facet(*value).to_owned())
        .collect::<Vec<_>>();
    output.sort();
    output
}

fn node_ref(value: &NodeRef) -> GraphNodeRefV1 {
    match value {
        NodeRef::Contract(id) => GraphNodeRefV1 {
            kind: "contract".to_owned(),
            id: id.as_str().to_owned(),
        },
        NodeRef::Representation(id) => GraphNodeRefV1 {
            kind: "representation".to_owned(),
            id: id.as_str().to_owned(),
        },
    }
}

fn facet(value: ContractFacet) -> &'static str {
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

fn representation_kind(value: RepresentationKind) -> &'static str {
    match value {
        RepresentationKind::ExecutableSurface => "executable-surface",
        RepresentationKind::SourceCode => "source-code",
        RepresentationKind::Schema => "schema",
        RepresentationKind::TypeDefinition => "type-definition",
        RepresentationKind::Validator => "validator",
        RepresentationKind::Test => "test",
        RepresentationKind::Documentation => "documentation",
        RepresentationKind::Configuration => "configuration",
        RepresentationKind::GeneratedArtifact => "generated-artifact",
        RepresentationKind::Other => "other",
    }
}

fn relation_kind(value: RelationKind) -> &'static str {
    match value {
        RelationKind::Defines => "defines",
        RelationKind::Implements => "implements",
        RelationKind::Documents => "documents",
        RelationKind::Validates => "validates",
        RelationKind::Generates => "generates",
        RelationKind::Projects => "projects",
        RelationKind::EquivalentTo => "equivalent-to",
        RelationKind::ConflictsWith => "conflicts-with",
        RelationKind::DependsOn => "depends-on",
    }
}

fn authority_basis(value: AuthorityBasis) -> &'static str {
    match value {
        AuthorityBasis::ExplicitDeclaration => "explicit-declaration",
        AuthorityBasis::MechanicalEnforcement => "mechanical-enforcement",
        AuthorityBasis::ObservedBehavior => "observed-behavior",
        AuthorityBasis::Documentation => "documentation",
        AuthorityBasis::Inference => "inference",
    }
}

fn clause_kind(value: ClauseKind) -> &'static str {
    match value {
        ClauseKind::Requirement => "requirement",
        ClauseKind::Guarantee => "guarantee",
        ClauseKind::Invariant => "invariant",
    }
}

fn clause_status(value: ClauseStatus) -> &'static str {
    match value {
        ClauseStatus::Consistent => "consistent",
        ClauseStatus::Contested => "contested",
    }
}
