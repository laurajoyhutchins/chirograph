use std::fs;
use std::path::Path;

use chirograph_core::model::ContractGraph;
use serde_json::Value;

use crate::{
    AnalysisError, AnalysisSourceContext, SourceFileKind, assemble_contract_graph,
    discover_sources, extract_json_schema_candidates, extract_rust_candidates,
};

pub fn analyze_tree(
    root: &Path,
    context: &AnalysisSourceContext,
) -> Result<ContractGraph, AnalysisError> {
    let discovered = discover_sources(root)?;
    let mut candidates = Vec::new();

    for source in discovered {
        let locator = relative_locator(&source.relative_path)?;
        let bytes = fs::read(root.join(&source.relative_path))?;
        match source.kind {
            SourceFileKind::Rust => {
                candidates.extend(extract_rust_candidates(context, &locator, &bytes)?);
            }
            SourceFileKind::Json if is_explicit_json_schema(&locator, &bytes) => {
                candidates.extend(extract_json_schema_candidates(context, &locator, &bytes)?);
            }
            SourceFileKind::Json => {}
        }
    }

    Ok(assemble_contract_graph(context, &candidates)?.graph)
}

fn relative_locator(path: &Path) -> Result<String, AnalysisError> {
    let value = path
        .to_str()
        .ok_or_else(|| AnalysisError::InvalidSourceRoot("source path is not UTF-8".into()))?;
    Ok(value.replace('\\', "/"))
}

fn is_explicit_json_schema(locator: &str, bytes: &[u8]) -> bool {
    let file_name = locator.rsplit('/').next().unwrap_or(locator);
    if file_name == "schema.json" || file_name.ends_with(".schema.json") {
        return true;
    }
    serde_json::from_slice::<Value>(bytes)
        .ok()
        .and_then(|value| value.get("$schema").cloned())
        .is_some()
}
