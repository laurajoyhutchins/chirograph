//! Versioned interchange for pre-alignment representation claims.

use serde::Deserialize;
use serde_json::Value;

use crate::alignment::{
    AlignmentCatalog, AlignmentClaim, AlignmentError, AlignmentState, ObservedRepresentation,
};
use crate::model::{
    ContractFacet, ContractGraph, ContractId, ObservationId, RepresentationId, RepresentationKind,
    SourceId,
};

pub const ALIGNMENT_SCHEMA_V1: &str = "chirograph-alignments-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlignmentInterchangeError {
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
    InvalidCatalog(AlignmentError),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct AlignmentDocumentV1 {
    schema: String,
    representations: Vec<ObservedRepresentationWire>,
    claims: Vec<AlignmentClaimWire>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct ObservedRepresentationWire {
    id: String,
    source: String,
    kind: String,
    locator: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct AlignmentClaimWire {
    representation: String,
    contract: String,
    facet: String,
    state: String,
    evidence: Vec<String>,
}

pub fn parse_alignment_json(
    input: &str,
    graph: &ContractGraph,
) -> Result<AlignmentCatalog, AlignmentInterchangeError> {
    let value: Value = serde_json::from_str(input)
        .map_err(|error| AlignmentInterchangeError::InvalidJson(error.to_string()))?;
    let schema = value
        .get("schema")
        .and_then(Value::as_str)
        .ok_or_else(|| AlignmentInterchangeError::InvalidJson("schema must be a string".into()))?;
    if schema != ALIGNMENT_SCHEMA_V1 {
        return Err(AlignmentInterchangeError::UnsupportedSchema(schema.into()));
    }

    let document: AlignmentDocumentV1 = serde_json::from_value(value)
        .map_err(|error| AlignmentInterchangeError::InvalidJson(error.to_string()))?;
    let catalog = document.into_catalog()?;
    catalog
        .validate_against(graph)
        .map_err(AlignmentInterchangeError::InvalidCatalog)?;
    Ok(catalog)
}

impl AlignmentDocumentV1 {
    fn into_catalog(self) -> Result<AlignmentCatalog, AlignmentInterchangeError> {
        Ok(AlignmentCatalog {
            representations: self
                .representations
                .into_iter()
                .map(|representation| {
                    Ok(ObservedRepresentation {
                        id: representation_id(representation.id)?,
                        source: source_id(representation.source)?,
                        kind: parse_representation_kind(&representation.kind)?,
                        locator: representation.locator,
                    })
                })
                .collect::<Result<Vec<_>, AlignmentInterchangeError>>()?,
            claims: self
                .claims
                .into_iter()
                .map(|claim| {
                    Ok(AlignmentClaim {
                        representation: representation_id(claim.representation)?,
                        contract: contract_id(claim.contract)?,
                        facet: parse_facet(&claim.facet)?,
                        state: parse_state(&claim.state)?,
                        evidence: claim
                            .evidence
                            .into_iter()
                            .map(observation_id)
                            .collect::<Result<Vec<_>, AlignmentInterchangeError>>()?,
                    })
                })
                .collect::<Result<Vec<_>, AlignmentInterchangeError>>()?,
        })
    }
}

fn identifier_error(
    kind: &str,
    value: String,
    error: impl std::fmt::Debug,
) -> AlignmentInterchangeError {
    AlignmentInterchangeError::InvalidIdentifier {
        kind: kind.into(),
        value,
        reason: format!("{error:?}"),
    }
}

fn contract_id(value: String) -> Result<ContractId, AlignmentInterchangeError> {
    ContractId::new(value.clone()).map_err(|error| identifier_error("contract", value, error))
}

fn representation_id(value: String) -> Result<RepresentationId, AlignmentInterchangeError> {
    RepresentationId::new(value.clone())
        .map_err(|error| identifier_error("representation", value, error))
}

fn source_id(value: String) -> Result<SourceId, AlignmentInterchangeError> {
    SourceId::new(value.clone()).map_err(|error| identifier_error("source", value, error))
}

fn observation_id(value: String) -> Result<ObservationId, AlignmentInterchangeError> {
    ObservationId::new(value.clone()).map_err(|error| identifier_error("observation", value, error))
}

fn invalid_value(field: &str, value: &str) -> AlignmentInterchangeError {
    AlignmentInterchangeError::InvalidValue {
        field: field.into(),
        value: value.into(),
    }
}

fn parse_facet(value: &str) -> Result<ContractFacet, AlignmentInterchangeError> {
    match value {
        "structural" => Ok(ContractFacet::Structural),
        "executable" => Ok(ContractFacet::Executable),
        "semantic" => Ok(ContractFacet::Semantic),
        "failure" => Ok(ContractFacet::Failure),
        "concurrency" => Ok(ContractFacet::Concurrency),
        "recovery" => Ok(ContractFacet::Recovery),
        "verification" => Ok(ContractFacet::Verification),
        _ => Err(invalid_value("claim.facet", value)),
    }
}

fn parse_state(value: &str) -> Result<AlignmentState, AlignmentInterchangeError> {
    match value {
        "confirmed" => Ok(AlignmentState::Confirmed),
        "rejected" => Ok(AlignmentState::Rejected),
        "unresolved" => Ok(AlignmentState::Unresolved),
        _ => Err(invalid_value("claim.state", value)),
    }
}

fn parse_representation_kind(value: &str) -> Result<RepresentationKind, AlignmentInterchangeError> {
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
