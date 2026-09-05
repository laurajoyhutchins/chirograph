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

#[derive(Debug, Clone)]
struct ClosedValueEvidence {
    values: BTreeSet<String>,
    start_byte: usize,
    end_byte: usize,
    fact: String,
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
    let declaration_attributes = leading_attributes(facts, declaration.fact, bytes);
    let rename_all = serde_rename_all(declaration_attributes.clone());
    let fields = fields_for(facts, declaration);

    if fields.len() == 1
        && fields[0].name.is_none()
        && derives_serialize(&declaration_attributes)
        && let Some(target_index) = unique_field_target(facts, fields[0], by_name)
        && declarations[target_index].fact.kind == RustFactKind::Struct
    {
        let target = &declarations[target_index];
        let mut evidence = inherited_evidence.to_vec();
        evidence.push(CandidateEvidence {
            source: context.source.clone(),
            revision: context.revision.clone(),
            locator: span_locator(locator, fields[0]),
            fact: format!(
                "transparent wrapper {} with unique serialized inner type edge to {}",
                qualified_identity(declaration),
                qualified_identity(target)
            ),
        });
        let result = walk_struct(
            target_index,
            path,
            &evidence,
            bytes,
            facts,
            declarations,
            by_name,
            context,
            locator,
            active,
            candidates,
        );
        active.remove(&declaration_index);
        return result;
    }

    for field in fields {
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
                let Some(closed) = enum_closed_values(facts, target, bytes) else {
                    continue;
                };
                evidence.push(CandidateEvidence {
                    source: context.source.clone(),
                    revision: context.revision.clone(),
                    locator: byte_span_locator(locator, closed.start_byte, closed.end_byte),
                    fact: closed.fact,
                });
                candidates.push(RepresentationCandidate::new(
                    RepresentationKind::SourceCode,
                    qualified_identity(target),
                    locator,
                    BTreeSet::from([ContractFacet::Structural]),
                    SemanticPath::new(next_path)?,
                    Some(closed.values),
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
    let mut type_facts = facts.iter().filter(|fact| {
        fact.kind == RustFactKind::TypeExpression
            && fact.container == field.container
            && fact.span.start_byte >= field.span.start_byte
            && fact.span.end_byte <= field.span.end_byte
    });
    let mut type_fact = type_facts.next()?;
    for candidate in type_facts {
        let current_width = type_fact
            .span
            .end_byte
            .saturating_sub(type_fact.span.start_byte);
        let candidate_width = candidate
            .span
            .end_byte
            .saturating_sub(candidate.span.start_byte);
        if candidate_width > current_width {
            type_fact = candidate;
        } else if candidate_width == current_width
            && (candidate.span.start_byte != type_fact.span.start_byte
                || candidate.span.end_byte != type_fact.span.end_byte)
        {
            return None;
        }
    }
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

fn derives_serialize(attributes: &[&RustFact]) -> bool {
    attributes.iter().any(|attribute| {
        let names = identifiers(&attribute.text);
        names.contains("derive") && names.contains("Serialize")
    })
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
) -> Option<ClosedValueEvidence> {
    let variants = enum_variant_names(facts, declaration)?;
    if let Some(rename_all) = serde_rename_all(leading_attributes(facts, declaration.fact, bytes)) {
        let values = variants
            .iter()
            .map(|variant| apply_case_rule(variant, &rename_all))
            .collect::<Option<BTreeSet<_>>>()?;
        return Some(ClosedValueEvidence {
            fact: format!(
                "closed serialized enum [{}]",
                values.iter().cloned().collect::<Vec<_>>().join(",")
            ),
            values,
            start_byte: declaration.fact.span.start_byte,
            end_byte: declaration.fact.span.end_byte,
        });
    }

    manual_deserialize_closed_string_vocabulary(facts, declaration, bytes, &variants)
}

fn enum_variant_names(
    facts: &[RustFact],
    declaration: &Declaration<'_>,
) -> Option<BTreeSet<String>> {
    let variants = facts
        .iter()
        .filter(|fact| {
            fact.kind == RustFactKind::Variant && fact.container == declaration.full_container
        })
        .map(|fact| fact.name.clone())
        .collect::<Option<BTreeSet<_>>>()?;
    (!variants.is_empty()).then_some(variants)
}

fn manual_deserialize_closed_string_vocabulary(
    facts: &[RustFact],
    declaration: &Declaration<'_>,
    bytes: &[u8],
    variants: &BTreeSet<String>,
) -> Option<ClosedValueEvidence> {
    let enum_name = declaration.fact.name.as_deref()?;
    let impls = facts
        .iter()
        .filter(|fact| {
            fact.kind == RustFactKind::Impl
                && fact.container == declaration.fact.container
                && impl_targets_declaration(facts, fact, enum_name)
                && impl_trait_name(facts, fact).as_deref() == Some("Deserialize")
        })
        .collect::<Vec<_>>();
    let [implementation] = impls.as_slice() else {
        return None;
    };

    let methods = facts
        .iter()
        .filter(|fact| {
            fact.kind == RustFactKind::Method
                && fact.name.as_deref() == Some("deserialize")
                && fact.container == declaration.full_container
                && contained_by(fact, implementation)
        })
        .collect::<Vec<_>>();
    let [method] = methods.as_slice() else {
        return None;
    };

    let mut match_container = declaration.full_container.clone();
    match_container.push("deserialize".to_owned());
    let candidates = facts
        .iter()
        .filter(|fact| {
            fact.kind == RustFactKind::Match
                && fact.container == match_container
                && contained_by(fact, method)
        })
        .filter_map(|fact| closed_string_match(facts, fact, bytes, enum_name, variants))
        .collect::<Vec<_>>();
    let [candidate] = candidates.as_slice() else {
        return None;
    };
    Some(candidate.clone())
}

fn impl_targets_declaration(facts: &[RustFact], implementation: &RustFact, name: &str) -> bool {
    let targets = facts
        .iter()
        .filter(|fact| {
            fact.kind == RustFactKind::TypeExpression
                && fact.container == implementation.container
                && contained_by(fact, implementation)
                && fact.text.trim() == name
        })
        .count();
    targets == 1
}

fn impl_trait_name(facts: &[RustFact], implementation: &RustFact) -> Option<String> {
    let references = facts
        .iter()
        .filter(|fact| {
            fact.kind == RustFactKind::TraitReference
                && fact.container == implementation.container
                && contained_by(fact, implementation)
        })
        .collect::<Vec<_>>();
    let [reference] = references.as_slice() else {
        return None;
    };
    let without_generics = reference.text.split('<').next()?.trim();
    let name = without_generics.rsplit("::").next()?.trim();
    (!name.is_empty() && name.chars().all(|character| character.is_ascii_alphanumeric() || character == '_'))
        .then(|| name.to_owned())
}

fn closed_string_match(
    facts: &[RustFact],
    match_fact: &RustFact,
    bytes: &[u8],
    enum_name: &str,
    variants: &BTreeSet<String>,
) -> Option<ClosedValueEvidence> {
    if facts.iter().any(|fact| {
        fact.kind == RustFactKind::Match
            && fact.container == match_fact.container
            && strictly_contained_by(fact, match_fact)
    }) {
        return None;
    }

    let arms = facts
        .iter()
        .filter(|fact| {
            fact.kind == RustFactKind::MatchArm
                && fact.container == match_fact.container
                && contained_by(fact, match_fact)
        })
        .collect::<Vec<_>>();
    if arms.is_empty() {
        return None;
    }

    let mut values = BTreeSet::new();
    let mut mapped_variants = BTreeSet::new();
    let mut rejecting_wildcards = 0usize;
    for arm in arms {
        let patterns = facts
            .iter()
            .filter(|fact| {
                fact.kind == RustFactKind::MatchPattern
                    && fact.container == arm.container
                    && contained_by(fact, arm)
            })
            .collect::<Vec<_>>();
        let [pattern] = patterns.as_slice() else {
            return None;
        };
        let body = std::str::from_utf8(bytes.get(pattern.span.end_byte..arm.span.end_byte)?)
            .ok()?;
        if pattern.text.trim() == "_" {
            if body.trim_start().starts_with("=> Err(") || body.contains("return Err(") {
                rejecting_wildcards += 1;
                continue;
            }
            return None;
        }

        let value = parse_simple_rust_string_pattern(&pattern.text)?;
        let variant = mapped_enum_variant(body, enum_name, variants)?;
        if !values.insert(value) {
            return None;
        }
        mapped_variants.insert(variant);
    }

    if rejecting_wildcards != 1 || values.is_empty() || &mapped_variants != variants {
        return None;
    }

    Some(ClosedValueEvidence {
        fact: format!(
            "manual Deserialize closed string vocabulary [{}]",
            values.iter().cloned().collect::<Vec<_>>().join(",")
        ),
        values,
        start_byte: match_fact.span.start_byte,
        end_byte: match_fact.span.end_byte,
    })
}

fn parse_simple_rust_string_pattern(text: &str) -> Option<String> {
    let value = serde_json::from_str::<String>(text.trim()).ok()?;
    (!value.is_empty() && value.trim() == value).then_some(value)
}

fn mapped_enum_variant(
    body: &str,
    enum_name: &str,
    variants: &BTreeSet<String>,
) -> Option<String> {
    let matches = variants
        .iter()
        .filter(|variant| {
            body.contains(&format!("Self::{variant}"))
                || body.contains(&format!("{enum_name}::{variant}"))
        })
        .cloned()
        .collect::<Vec<_>>();
    let [variant] = matches.as_slice() else {
        return None;
    };
    Some(variant.clone())
}

fn contained_by(inner: &RustFact, outer: &RustFact) -> bool {
    inner.span.start_byte >= outer.span.start_byte && inner.span.end_byte <= outer.span.end_byte
}

fn strictly_contained_by(inner: &RustFact, outer: &RustFact) -> bool {
    contained_by(inner, outer)
        && (inner.span.start_byte != outer.span.start_byte
            || inner.span.end_byte != outer.span.end_byte)
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
    byte_span_locator(locator, fact.span.start_byte, fact.span.end_byte)
}

fn byte_span_locator(locator: &str, start_byte: usize, end_byte: usize) -> String {
    format!("{locator}#bytes={start_byte}-{end_byte}")
}
