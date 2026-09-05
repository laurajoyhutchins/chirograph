use std::collections::BTreeSet;

use chirograph_core::model::{ContractFacet, RepresentationKind};
use serde_json::Value;

use crate::{
    AnalysisError, AnalysisSourceContext, CandidateEvidence, CandidateMechanism,
    RepresentationCandidate, SemanticPath,
};

pub fn extract_json_schema_candidates(
    context: &AnalysisSourceContext,
    locator: &str,
    bytes: &[u8],
) -> Result<Vec<RepresentationCandidate>, AnalysisError> {
    if locator.is_empty() || locator.trim() != locator {
        return Err(AnalysisError::InvalidSchema(
            "schema locator must be nonempty and trimmed".into(),
        ));
    }
    let root: Value = serde_json::from_slice(bytes)
        .map_err(|error| AnalysisError::InvalidSchema(error.to_string()))?;
    if !root.is_object() {
        return Err(AnalysisError::InvalidSchema(
            "schema document root must be an object".into(),
        ));
    }

    let mut candidates = Vec::new();
    walk_node(
        &root,
        &root,
        context,
        locator,
        &[],
        "#",
        &BTreeSet::new(),
        &mut BTreeSet::new(),
        &mut candidates,
    )?;
    candidates.sort_by(|left, right| {
        left.semantic_path
            .cmp(&right.semantic_path)
            .then_with(|| {
                left.qualified_local_identity
                    .cmp(&right.qualified_local_identity)
            })
            .then_with(|| left.locator.cmp(&right.locator))
    });
    candidates.dedup();
    Ok(candidates)
}

#[allow(clippy::too_many_arguments)]
fn walk_node(
    root: &Value,
    node: &Value,
    context: &AnalysisSourceContext,
    locator: &str,
    path: &[String],
    pointer: &str,
    mechanisms: &BTreeSet<CandidateMechanism>,
    active_refs: &mut BTreeSet<String>,
    candidates: &mut Vec<RepresentationCandidate>,
) -> Result<(), AnalysisError> {
    let object = node.as_object().ok_or_else(|| {
        AnalysisError::InvalidSchema(format!("schema node at {pointer} must be an object"))
    })?;

    if let Some(reference) = object.get("$ref") {
        let reference = reference.as_str().ok_or_else(|| {
            AnalysisError::InvalidSchema(format!("$ref at {pointer} must be a string"))
        })?;
        if !reference.starts_with("#/") {
            return Err(AnalysisError::InvalidSchema(format!(
                "unsupported non-local $ref at {pointer}: {reference}"
            )));
        }
        if !active_refs.insert(reference.to_owned()) {
            return Err(AnalysisError::InvalidSchema(format!(
                "cyclic local $ref: {reference}"
            )));
        }
        let target = root.pointer(&reference[1..]).ok_or_else(|| {
            AnalysisError::InvalidSchema(format!("broken local $ref: {reference}"))
        })?;
        let mut next_mechanisms = mechanisms.clone();
        next_mechanisms.insert(CandidateMechanism::JsonSchemaReference);
        let result = walk_node(
            root,
            target,
            context,
            locator,
            path,
            reference,
            &next_mechanisms,
            active_refs,
            candidates,
        );
        active_refs.remove(reference);
        return result;
    }

    if let Some(any_of) = object.get("anyOf") {
        let branches = any_of.as_array().ok_or_else(|| {
            AnalysisError::InvalidSchema(format!("anyOf at {pointer} must be an array"))
        })?;
        let substantive = branches
            .iter()
            .enumerate()
            .filter(|(_, branch)| !is_null_schema(branch))
            .collect::<Vec<_>>();
        if substantive.len() != 1 {
            return Ok(());
        }
        let (index, branch) = substantive[0];
        return walk_node(
            root,
            branch,
            context,
            locator,
            path,
            &format!("{pointer}/anyOf/{index}"),
            mechanisms,
            active_refs,
            candidates,
        );
    }

    if let Some(enum_values) = object.get("enum") {
        let enum_values = enum_values.as_array().ok_or_else(|| {
            AnalysisError::InvalidSchema(format!("enum at {pointer} must be an array"))
        })?;
        if path.is_empty() {
            return Ok(());
        }
        let mut values = BTreeSet::new();
        for value in enum_values {
            let Some(value) = value.as_str() else {
                return Ok(());
            };
            if value.is_empty() {
                return Ok(());
            }
            values.insert(value.to_owned());
        }
        if values.is_empty() {
            return Ok(());
        }

        let semantic_path = SemanticPath::new(path.iter().cloned())?;
        let mut candidate_mechanisms = mechanisms.clone();
        candidate_mechanisms.insert(CandidateMechanism::JsonSchemaProperty);
        candidate_mechanisms.insert(CandidateMechanism::JsonSchemaClosedValueSet);
        let evidence_locator = format!("{locator}{pointer}");
        let evidence = vec![
            CandidateEvidence {
                source: context.source.clone(),
                revision: context.revision.clone(),
                locator: evidence_locator.clone(),
                fact: format!("explicit schema property path {}", semantic_path.dotted()),
            },
            CandidateEvidence {
                source: context.source.clone(),
                revision: context.revision.clone(),
                locator: evidence_locator,
                fact: format!(
                    "closed string enum [{}]",
                    values.iter().cloned().collect::<Vec<_>>().join(",")
                ),
            },
        ];
        candidates.push(RepresentationCandidate::new(
            RepresentationKind::Schema,
            pointer,
            locator,
            BTreeSet::from([ContractFacet::Structural]),
            semantic_path,
            Some(values),
            candidate_mechanisms,
            evidence,
        )?);
        return Ok(());
    }

    if let Some(properties) = object.get("properties") {
        let properties = properties.as_object().ok_or_else(|| {
            AnalysisError::InvalidSchema(format!("properties at {pointer} must be an object"))
        })?;
        let mut keys = properties.keys().collect::<Vec<_>>();
        keys.sort();
        for key in keys {
            let mut next_path = path.to_vec();
            next_path.push(key.clone());
            let mut next_mechanisms = mechanisms.clone();
            next_mechanisms.insert(CandidateMechanism::JsonSchemaProperty);
            walk_node(
                root,
                &properties[key],
                context,
                locator,
                &next_path,
                &format!("{pointer}/properties/{}", escape_pointer_segment(key)),
                &next_mechanisms,
                active_refs,
                candidates,
            )?;
        }
    }

    if let Some(additional) = object.get("additionalProperties")
        && additional.is_object()
    {
        walk_node(
            root,
            additional,
            context,
            locator,
            path,
            &format!("{pointer}/additionalProperties"),
            mechanisms,
            active_refs,
            candidates,
        )?;
    }

    Ok(())
}

fn is_null_schema(value: &Value) -> bool {
    match value.get("type") {
        Some(Value::String(value)) => value == "null",
        Some(Value::Array(values)) => {
            !values.is_empty()
                && values
                    .iter()
                    .all(|value| value.as_str().is_some_and(|value| value == "null"))
        }
        _ => false,
    }
}

fn escape_pointer_segment(segment: &str) -> String {
    segment.replace('~', "~0").replace('/', "~1")
}
