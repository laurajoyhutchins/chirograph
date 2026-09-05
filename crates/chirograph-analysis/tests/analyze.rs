use std::fs;
use std::path::PathBuf;

use chirograph_analysis::{AnalysisSourceContext, analyze_tree};
use chirograph_core::model::Revision;

const REVISION: &str = "0123456789abcdef0123456789abcdef01234567";

fn fixture_tree(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "chirograph-analysis-tree-{}-{label}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("manifest.rs"),
        r#"
#[serde(rename_all = "kebab-case")]
struct Manifest {
    profile: Profile,
}

#[serde(rename_all = "kebab-case")]
struct Profile {
    debug_info: DebugInfo,
}

#[serde(rename_all = "kebab-case")]
enum DebugInfo {
    None,
    Full,
}
"#,
    )
    .unwrap();
    fs::write(
        root.join("manifest.schema.json"),
        r#"{"properties":{"profile":{"properties":{"debug-info":{"enum":["None","Full"]}}}}}"#,
    )
    .unwrap();
    root
}

#[test]
fn analyze_tree_discovers_extracts_aligns_and_assembles() {
    let root = fixture_tree("pipeline");
    let context = AnalysisSourceContext::github(
        "acme/fixture-project",
        Revision::Exact(REVISION.into()),
    )
    .unwrap();

    let graph = analyze_tree(&root, &context).unwrap();
    fs::remove_dir_all(&root).unwrap();

    graph.validate().unwrap();
    assert_eq!(graph.contracts.len(), 1);
    assert_eq!(graph.contracts[0].id.as_str(), "fixture-project.profile.debug-info");
}
