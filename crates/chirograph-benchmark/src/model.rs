use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use chirograph_core::graph_json::GraphNodeRefV1;
use serde::{Deserialize, Serialize};

pub const SPECIMEN_SCHEMA: &str = "chirograph-benchmark-specimen-v1";
pub const GOLDEN_SCHEMA: &str = "chirograph-benchmark-golden-v1";

const FACETS: &[&str] = &[
    "structural",
    "executable",
    "semantic",
    "failure",
    "concurrency",
    "recovery",
    "verification",
];
const REPRESENTATION_KINDS: &[&str] = &[
    "executable-surface",
    "source-code",
    "schema",
    "type-definition",
    "validator",
    "test",
    "documentation",
    "configuration",
    "generated-artifact",
    "other",
];
const RELATION_KINDS: &[&str] = &[
    "defines",
    "implements",
    "documents",
    "validates",
    "generates",
    "projects",
    "equivalent-to",
    "conflicts-with",
    "depends-on",
];
const AUTHORITY_BASES: &[&str] = &[
    "explicit-declaration",
    "mechanical-enforcement",
    "observed-behavior",
    "documentation",
    "inference",
];
const CLAUSE_KINDS: &[&str] = &["requirement", "guarantee", "invariant"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpecimenV1 {
    pub schema: String,
    pub id: String,
    pub repository: String,
    pub scenario: String,
    pub upstream: UpstreamV1,
    pub files: Vec<FixtureFileV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpstreamV1 {
    pub repository: String,
    pub revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureFileV1 {
    pub fixture_path: String,
    pub upstream_path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoldenV1 {
    pub schema: String,
    pub contracts: Vec<GoldenContractV1>,
    pub representations: Vec<GoldenRepresentationV1>,
    pub authority_claims: Vec<GoldenAuthorityClaimV1>,
    pub relationships: Vec<GoldenRelationshipV1>,
    pub clauses: Vec<GoldenClauseV1>,
    pub lifecycle: Vec<GoldenLifecycleV1>,
    pub expected_findings: Vec<GoldenFindingV1>,
    pub non_contracts: Vec<GoldenNonContractV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoldenContractV1 {
    pub id: String,
    pub facets: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoldenRepresentationV1 {
    pub id: String,
    pub contract: String,
    pub kind: String,
    pub locator: String,
    pub facets: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoldenAuthorityClaimV1 {
    pub contract: String,
    pub facet: String,
    pub representation: String,
    pub basis: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoldenRelationshipV1 {
    pub from: GraphNodeRefV1,
    pub kind: String,
    pub to: GraphNodeRefV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoldenClauseV1 {
    pub id: String,
    pub contract: String,
    pub facet: String,
    pub kind: String,
    pub statement: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoldenLifecycleV1 {
    pub subject: GraphNodeRefV1,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum GoldenFindingV1 {
    ContestedClause { clause: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoldenNonContractV1 {
    pub locator: String,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct BenchmarkCase {
    pub id: String,
    pub repository: String,
    pub scenario: String,
    pub root: PathBuf,
    pub fixture_dir: PathBuf,
    pub specimen_path: PathBuf,
    pub golden_path: PathBuf,
    pub specimen: SpecimenV1,
    pub golden: GoldenV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkModelError {
    message: String,
}

impl BenchmarkModelError {
    fn invalid(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for BenchmarkModelError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for BenchmarkModelError {}

pub fn parse_specimen_yaml(input: &str) -> Result<SpecimenV1, BenchmarkModelError> {
    let value: SpecimenV1 = yaml_serde::from_str(input)
        .map_err(|error| BenchmarkModelError::invalid(format!("invalid specimen YAML: {error}")))?;
    validate_specimen(&value)?;
    Ok(value)
}

pub fn parse_golden_yaml(input: &str) -> Result<GoldenV1, BenchmarkModelError> {
    let value: GoldenV1 = yaml_serde::from_str(input)
        .map_err(|error| BenchmarkModelError::invalid(format!("invalid golden YAML: {error}")))?;
    validate_golden(&value)?;
    Ok(value)
}

pub fn validate_specimen(value: &SpecimenV1) -> Result<(), BenchmarkModelError> {
    if value.schema != SPECIMEN_SCHEMA {
        return Err(BenchmarkModelError::invalid(format!(
            "unsupported specimen schema: {}",
            value.schema
        )));
    }
    require_identifier(&value.id, "specimen.id")?;
    require_identifier(&value.repository, "specimen.repository")?;
    require_identifier(&value.scenario, "specimen.scenario")?;
    require_identifier(&value.upstream.repository, "specimen.upstream.repository")?;
    if !is_exact_revision(&value.upstream.revision) {
        return Err(BenchmarkModelError::invalid(
            "specimen.upstream.revision must be exactly 40 hexadecimal characters",
        ));
    }
    if value.files.is_empty() {
        return Err(BenchmarkModelError::invalid(
            "specimen.files must contain at least one fixture",
        ));
    }

    let mut fixture_paths = BTreeSet::new();
    for file in &value.files {
        validate_relative_path(&file.fixture_path, "fixture_path", Some("fixture/"))?;
        validate_relative_path(&file.upstream_path, "upstream_path", None)?;
        if !is_sha256(&file.sha256) {
            return Err(BenchmarkModelError::invalid(format!(
                "invalid sha256 for {}",
                file.fixture_path
            )));
        }
        if !fixture_paths.insert(file.fixture_path.as_str()) {
            return Err(BenchmarkModelError::invalid(format!(
                "duplicate fixture path: {}",
                file.fixture_path
            )));
        }
    }
    Ok(())
}

pub fn validate_golden(value: &GoldenV1) -> Result<(), BenchmarkModelError> {
    if value.schema != GOLDEN_SCHEMA {
        return Err(BenchmarkModelError::invalid(format!(
            "unsupported golden schema: {}",
            value.schema
        )));
    }
    if value.contracts.is_empty() {
        return Err(BenchmarkModelError::invalid(
            "golden truth must contain at least one logical contract",
        ));
    }

    let mut contracts = BTreeMap::new();
    for contract in &value.contracts {
        require_identifier(&contract.id, "contract.id")?;
        validate_vocab(&contract.facets, FACETS, "contract facet")?;
        if contracts.insert(contract.id.as_str(), contract).is_some() {
            return Err(BenchmarkModelError::invalid(format!(
                "duplicate contract id: {}",
                contract.id
            )));
        }
    }

    let mut representations = BTreeMap::new();
    for representation in &value.representations {
        require_identifier(&representation.id, "representation.id")?;
        require_identifier(&representation.locator, "representation.locator")?;
        let contract = contracts
            .get(representation.contract.as_str())
            .ok_or_else(|| {
                BenchmarkModelError::invalid(format!(
                    "unknown representation contract: {}",
                    representation.contract
                ))
            })?;
        if !REPRESENTATION_KINDS.contains(&representation.kind.as_str()) {
            return Err(BenchmarkModelError::invalid(format!(
                "unsupported representation kind: {}",
                representation.kind
            )));
        }
        validate_vocab(&representation.facets, FACETS, "representation facet")?;
        for facet in &representation.facets {
            if !contract.facets.contains(facet) {
                return Err(BenchmarkModelError::invalid(format!(
                    "representation facet {facet} is outside contract {}",
                    representation.contract
                )));
            }
        }
        if representations
            .insert(representation.id.as_str(), representation)
            .is_some()
        {
            return Err(BenchmarkModelError::invalid(format!(
                "duplicate representation id: {}",
                representation.id
            )));
        }
    }

    let mut clauses = BTreeSet::new();
    for clause in &value.clauses {
        require_identifier(&clause.id, "clause.id")?;
        require_identifier(&clause.statement, "clause.statement")?;
        let contract = contracts.get(clause.contract.as_str()).ok_or_else(|| {
            BenchmarkModelError::invalid(format!("unknown clause contract: {}", clause.contract))
        })?;
        if !FACETS.contains(&clause.facet.as_str()) || !contract.facets.contains(&clause.facet) {
            return Err(BenchmarkModelError::invalid(format!(
                "invalid clause facet {} for contract {}",
                clause.facet, clause.contract
            )));
        }
        if !CLAUSE_KINDS.contains(&clause.kind.as_str()) {
            return Err(BenchmarkModelError::invalid(format!(
                "unsupported clause kind: {}",
                clause.kind
            )));
        }
        if !clauses.insert(clause.id.as_str()) {
            return Err(BenchmarkModelError::invalid(format!(
                "duplicate clause id: {}",
                clause.id
            )));
        }
    }

    for claim in &value.authority_claims {
        let representation = representations
            .get(claim.representation.as_str())
            .ok_or_else(|| {
                BenchmarkModelError::invalid(format!(
                    "unknown authority representation: {}",
                    claim.representation
                ))
            })?;
        if representation.contract != claim.contract
            || !contracts.contains_key(claim.contract.as_str())
        {
            return Err(BenchmarkModelError::invalid(format!(
                "authority contract mismatch for representation {}",
                claim.representation
            )));
        }
        if !FACETS.contains(&claim.facet.as_str()) || !representation.facets.contains(&claim.facet)
        {
            return Err(BenchmarkModelError::invalid(format!(
                "invalid authority facet: {}",
                claim.facet
            )));
        }
        if !AUTHORITY_BASES.contains(&claim.basis.as_str()) {
            return Err(BenchmarkModelError::invalid(format!(
                "unsupported authority basis: {}",
                claim.basis
            )));
        }
    }

    for relationship in &value.relationships {
        validate_node(&relationship.from, &contracts, &representations)?;
        validate_node(&relationship.to, &contracts, &representations)?;
        if !RELATION_KINDS.contains(&relationship.kind.as_str()) {
            return Err(BenchmarkModelError::invalid(format!(
                "unsupported relationship kind: {}",
                relationship.kind
            )));
        }
    }

    for lifecycle in &value.lifecycle {
        validate_node(&lifecycle.subject, &contracts, &representations)?;
        require_identifier(&lifecycle.status, "lifecycle.status")?;
    }

    for finding in &value.expected_findings {
        match finding {
            GoldenFindingV1::ContestedClause { clause } if !clauses.contains(clause.as_str()) => {
                return Err(BenchmarkModelError::invalid(format!(
                    "finding references unknown clause: {clause}"
                )));
            }
            GoldenFindingV1::ContestedClause { .. } => {}
        }
    }

    for non_contract in &value.non_contracts {
        require_identifier(&non_contract.locator, "non_contract.locator")?;
        require_identifier(&non_contract.reason, "non_contract.reason")?;
    }

    Ok(())
}

fn validate_node<'a>(
    node: &GraphNodeRefV1,
    contracts: &BTreeMap<&'a str, &'a GoldenContractV1>,
    representations: &BTreeMap<&'a str, &'a GoldenRepresentationV1>,
) -> Result<(), BenchmarkModelError> {
    match node.kind.as_str() {
        "contract" if contracts.contains_key(node.id.as_str()) => Ok(()),
        "representation" if representations.contains_key(node.id.as_str()) => Ok(()),
        "contract" | "representation" => Err(BenchmarkModelError::invalid(format!(
            "unknown {} node: {}",
            node.kind, node.id
        ))),
        _ => Err(BenchmarkModelError::invalid(format!(
            "unsupported node kind: {}",
            node.kind
        ))),
    }
}

fn validate_vocab(
    values: &[String],
    allowed: &[&str],
    field: &str,
) -> Result<(), BenchmarkModelError> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !allowed.contains(&value.as_str()) {
            return Err(BenchmarkModelError::invalid(format!(
                "unsupported {field}: {value}"
            )));
        }
        if !seen.insert(value) {
            return Err(BenchmarkModelError::invalid(format!(
                "duplicate {field}: {value}"
            )));
        }
    }
    Ok(())
}

fn require_identifier(value: &str, field: &str) -> Result<(), BenchmarkModelError> {
    if value.is_empty() || value.trim() != value {
        return Err(BenchmarkModelError::invalid(format!(
            "{field} must be non-empty without surrounding whitespace"
        )));
    }
    Ok(())
}

fn is_exact_revision(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_relative_path(
    value: &str,
    field: &str,
    required_prefix: Option<&str>,
) -> Result<(), BenchmarkModelError> {
    if value.is_empty()
        || value.starts_with('/')
        || value.starts_with('\\')
        || value
            .split(['/', '\\'])
            .any(|part| part == ".." || part.is_empty())
        || required_prefix.is_some_and(|prefix| !value.starts_with(prefix))
    {
        return Err(BenchmarkModelError::invalid(format!(
            "{field} must be a safe relative path"
        )));
    }
    Ok(())
}
