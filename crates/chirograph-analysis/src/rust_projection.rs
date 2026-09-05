use std::collections::{BTreeMap, BTreeSet};

use chirograph_core::model::{ContractFacet, RepresentationKind};
use chirograph_rust::{RustFact, RustFactKind, extract_rust_facts};
use chirograph_tree_sitter::SourceProvenance;

use crate::{
    AnalysisError, AnalysisSourceContext, CandidateEvidence, CandidateMechanism,
    RepresentationCandidate, SemanticPath,
};

#[derive(Debug, Clone)]
struct Declaration<'a> {
    fact: &'a RustFact,
    full_container: Vec<String>,
}

pub fn extract_rust_candidates(
    context: &AnalysisSourceContext,
    locator: &str,
    bytes: &[u8],
) -> Result<Vec<RepresentationCandidate>, AnalysisError> {
    if locator.is_empty() || locator.trim() != locator {
        return Err(AnalysisError::InvalidRustProjection(
            "Rust locator must be nonempty and trimmed".into(),
        ));
    }
    let provenance = SourceProvenance {
        source: context.source.clone(),
        revision: context.revision.clone(),
        locator: locator.to_owned(),
        path: locator.to_owned(),
    };
    let extraction = extract_rust_facts(bytes, provenance)
        .map_err(|error| AnalysisError::InvalidRustProjection(error.to_string()))?;
    if !extraction.diagnostics.is_empty() {
        return Err(AnalysisError::InvalidRustProjection(format!(
            "Rust source has {} parse diagnostics",
            extraction.diagnostics.len()
        )));
    }

    let declarations = collect_declarations(&extraction.facts);
    let by_name = declarations_by_simple_name(&declarations);
    let referenced = referenced_declarations(&extraction.facts, &declarations, &by_name);
    let mut roots = declarations
        .iter()
        .enumerate()
        .filter(|(index, declaration)| {
            declaration.fact.kind == RustFactKind::Struct && !referenced.contains(index)
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    roots.sort_by_key(|index| qualified_identity(&declarations[*index]));

    let mut candidates = Vec::new();
    for root in roots {
        walk_struct(
            root,
            &[],
            &[],
            bytes,
            &extraction.facts,
            &declarations,
            &by_name,
            context,
            locator,
            &mut BTreeSet::new(),
            &mut candidates,
        )?;
    }
    candidates.sort_by(|left, right| {
        left.semantic_path.cmp(&right.semantic_path).then_with(|| {
            left.qualified_local_identity
                .cmp(&right.qualified_local_identity)
        })
    });
    candidates.dedup();
    Ok(candidates)
}

fn collect_declarations(facts: &[RustFact]) -> Vec<Declaration<'_>> {
    facts
        .iter()
        .filter(|fact| matches!(fact.kind, RustFactKind::Struct | RustFactKind::Enum))
        .filter_map(|fact| {
            let name = fact.name.as_ref()?;
            let mut full_container = fact.container.clone();
            full_container.push(name.clone());
            Some(Declaration {
                fact,
                full_container,
            })
        })
        .collect()
}

fn declarations_by_simple_name(declarations: &[Declaration<'_>]) -> BTreeMap<String, Vec<usize>> {
    let mut by_name = BTreeMap::<String, Vec<usize>>::new();
    for (index, declaration) in declarations.iter().enumerate() {
        if let Some(name) = &declaration.fact.name {
            by_name.entry(name.clone()).or_default().push(index);
        }
    }
    by_name
}

fn referenced_declarations(
    facts: &[RustFact],
    declarations: &[Declaration<'_>],
    by_name: &BTreeMap<String, Vec<usize>>,
) -> BTreeSet<usize> {
    let mut referenced = BTreeSet::new();
    for declaration in declarations {
        if declaration.fact.kind != RustFactKind::Struct {
            continue;
        }
        for field in fields_for(facts, declaration) {
            if let Some(target) = unique_field_target(facts, field, by_name) {
                referenced.insert(target);
            }
        }
    }
    referenced
}

#[allow(clippy::too_many_arguments)]
fn walk_struct(
    declaration_index: usize,
    path: &[String],
    inherited_evidence: &[CandidateEvidence],
    bytes: &[u8],
    facts: &[RustFact],
    declarations: &[Declaration<'_>],
    by_name: &BTreeMap<String, Vec<usize>>,
    context: &AnalysisSourceContext,
    locator: &str,
    active: &mut BTreeSet<usize>,
    candidates: &mut Vec<RepresentationCandidate>,
) -> Result<(), AnalysisError> {
    if !active.insert(declaration_index) {
        return Ok(());
    }
    let declaration = &declarations[declaration_index];
    let rename_all = serde_rename_all(leading_attributes(facts, declaration.fact, bytes));

    for field in fields_for(facts, declaration) {
        let Some(field_name) = field.name.as_deref() else {
            continue;
        };
        let field_attributes = leading_attributes(facts, field, bytes);
        let Some(serialized_name) =
            serialized_name(field_name, &field_attributes, rename_all.as_deref())
        else {
            continue;
        };
        let Some(target_index) = unique_field_target(facts, field, by_name) else {
            continue;
        };
        let target = &declarations[target_index];
        let mut next_path = path.to_vec();
        next_path.push(serialized_name.clone());
        let mut evidence = inherited_evidence.to_vec();
        evidence.push(CandidateEvidence {
            source: context.source.clone(),
            revision: context.revision.clone(),
            locator: span_locator(locator, field),
            fact: format!(
                "serialized field {field_name} as {serialized_name} with unique type edge to {}",
                qualified_identity(target)
            ),
        });

        match target.fact.kind {
            RustFactKind::Struct => walk_struct(
                target_index,
                &next_path,
                &evidence,
                bytes,
                facts,
                declarations,
                by_name,
                context,
                locator,
                active,
                candidates,
            )?,
            RustFactKind::Enum => {
                let Some(values) = enum_closed_values(facts, target, bytes) else {
                    continue;
                };
                evidence.push(CandidateEvidence {
                    source: context.source.clone(),
                    revision: context.revision.clone(),
                    locator: span_locator(locator, target.fact),
                    fact: format!(
                        "closed serialized enum [{}]",
                        values.iter().cloned().collect::<Vec<_>>().join(",")
                    ),
                });
                candidates.push(RepresentationCandidate::new(
                    RepresentationKind::SourceCode,
                    qualified_identity(target),
                    locator,
                    BTreeSet::from([ContractFacet::Structural]),
                    SemanticPath::new(next_path)?,
                    Some(values),
                    BTreeSet::from([
                        CandidateMechanism::RustSerializedField,
                        CandidateMechanism::RustTypeReference,
                        CandidateMechanism::RustClosedValueSet,
                    ]),
                    evidence,
                )?);
            }
            _ => {}
        }
    }

    active.remove(&declaration_index);
    Ok(())
}

fn fields_for<'a>(facts: &'a [RustFact], declaration: &Declaration<'_>) -> Vec<&'a RustFact> {
    facts
        .iter()
        .filter(|fact| {
            fact.kind == RustFactKind::Field && fact.container == declaration.full_container
        })
        .collect()
}

fn unique_field_target(
    facts: &[RustFact],
    field: &RustFact,
    by_name: &BTreeMap<String, Vec<usize>>,
) -> Option<usize> {
    let type_fact = facts.iter().find(|fact| {
        fact.kind == RustFactKind::TypeExpression
            && fact.container == field.container
            && fact.span.start_byte >= field.span.start_byte
            && fact.span.end_byte <= field.span.end_byte
    })?;
    let identifiers = identifiers(&type_fact.text);
    let mut resolved = BTreeSet::new();
    for identifier in identifiers {
        let Some(matches) = by_name.get(&identifier) else {
            continue;
        };
        if matches.len() != 1 {
            return None;
        }
        resolved.insert(matches[0]);
    }
    if resolved.len() == 1 {
        resolved.into_iter().next()
    } else {
        None
    }
}

fn leading_attributes<'a>(
    facts: &'a [RustFact],
    owner: &RustFact,
    bytes: &[u8],
) -> Vec<&'a RustFact> {
    let mut candidates = facts
        .iter()
        .filter(|fact| {
            fact.kind == RustFactKind::Attribute
                && fact.container == owner.container
                && fact.span.end_byte <= owner.span.start_byte
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|fact| fact.span.start_byte);

    let mut cursor = owner.span.start_byte;
    let mut leading = Vec::new();
    for attribute in candidates.into_iter().rev() {
        if !whitespace_only(bytes, attribute.span.end_byte, cursor) {
            break;
        }
        leading.push(attribute);
        cursor = attribute.span.start_byte;
    }
    leading.reverse();
    leading
}

fn whitespace_only(bytes: &[u8], start: usize, end: usize) -> bool {
    start <= end
        && end <= bytes.len()
        && bytes[start..end]
            .iter()
            .all(|byte| byte.is_ascii_whitespace())
}

fn serde_rename_all(attributes: Vec<&RustFact>) -> Option<String> {
    attributes
        .into_iter()
        .find_map(|attribute| serde_string_argument(&attribute.text, "rename_all"))
}

fn serialized_name(
    field_name: &str,
    field_attributes: &[&RustFact],
    rename_all: Option<&str>,
) -> Option<String> {
    if let Some(explicit) = field_attributes
        .iter()
        .find_map(|attribute| serde_string_argument(&attribute.text, "rename"))
    {
        return Some(explicit);
    }
    rename_all.and_then(|rule| apply_case_rule(field_name, rule))
}

fn enum_closed_values(
    facts: &[RustFact],
    declaration: &Declaration<'_>,
    bytes: &[u8],
) -> Option<BTreeSet<String>> {
    let rename_all = serde_rename_all(leading_attributes(facts, declaration.fact, bytes))?;
    let variants = facts
        .iter()
        .filter(|fact| {
            fact.kind == RustFactKind::Variant && fact.container == declaration.full_container
        })
        .collect::<Vec<_>>();
    if variants.is_empty() {
        return None;
    }
    variants
        .into_iter()
        .map(|variant| {
            let name = variant.name.as_deref()?;
            apply_case_rule(name, &rename_all)
        })
        .collect()
}

fn serde_string_argument(text: &str, key: &str) -> Option<String> {
    if !text.contains("serde") {
        return None;
    }
    let key_index = text.find(key)? + key.len();
    let rest = &text[key_index..];
    let equals = rest.find('=')?;
    let rest = rest[equals + 1..].trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_owned())
}

fn apply_case_rule(value: &str, rule: &str) -> Option<String> {
    let words = split_words(value);
    if words.is_empty() {
        return None;
    }
    match rule {
        "kebab-case" => Some(words.join("-")),
        "snake_case" => Some(words.join("_")),
        "SCREAMING_SNAKE_CASE" => Some(words.join("_").to_ascii_uppercase()),
        "lowercase" => Some(words.concat()),
        "UPPERCASE" => Some(words.concat().to_ascii_uppercase()),
        "camelCase" => {
            let mut iter = words.into_iter();
            let mut result = iter.next()?;
            for word in iter {
                result.push_str(&capitalize(&word));
            }
            Some(result)
        }
        "PascalCase" => Some(words.into_iter().map(|word| capitalize(&word)).collect()),
        _ => None,
    }
}

fn split_words(value: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let chars = value.chars().collect::<Vec<_>>();
    for (index, character) in chars.iter().copied().enumerate() {
        if character == '_' || character == '-' {
            if !current.is_empty() {
                words.push(current.to_ascii_lowercase());
                current.clear();
            }
            continue;
        }
        let boundary = character.is_ascii_uppercase()
            && !current.is_empty()
            && (chars
                .get(index.wrapping_sub(1))
                .is_some_and(|previous| previous.is_ascii_lowercase())
                || chars
                    .get(index + 1)
                    .is_some_and(|next| next.is_ascii_lowercase()));
        if boundary {
            words.push(current.to_ascii_lowercase());
            current.clear();
        }
        current.push(character);
    }
    if !current.is_empty() {
        words.push(current.to_ascii_lowercase());
    }
    words
}

fn capitalize(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    format!("{}{}", first.to_ascii_uppercase(), chars.as_str())
}

fn identifiers(value: &str) -> BTreeSet<String> {
    let mut identifiers = BTreeSet::new();
    let mut current = String::new();
    for character in value.chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            current.push(character);
        } else if !current.is_empty() {
            identifiers.insert(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        identifiers.insert(current);
    }
    identifiers
}

fn qualified_identity(declaration: &Declaration<'_>) -> String {
    declaration.full_container.join("::")
}

fn span_locator(locator: &str, fact: &RustFact) -> String {
    format!(
        "{locator}#bytes={}-{}",
        fact.span.start_byte, fact.span.end_byte
    )
}
