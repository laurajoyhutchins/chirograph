use std::fs;
use std::path::PathBuf;

use chirograph_analysis::{AnalysisSourceContext, SourceFileKind, discover_sources};
use chirograph_core::model::Revision;

#[test]
fn derives_source_identity_and_namespace_from_explicit_repository_context() {
    let revision = Revision::Exact("0123456789abcdef0123456789abcdef01234567".into());
    let context = AnalysisSourceContext::github("acme/fixture-project", revision.clone()).unwrap();
    assert_eq!(context.repository, "acme/fixture-project");
    assert_eq!(context.source.as_str(), "github:acme/fixture-project");
    assert_eq!(context.namespace, "fixture-project");
    assert_eq!(context.revision, revision);

    assert!(AnalysisSourceContext::github("missing-owner", Revision::Unknown).is_err());
    assert!(AnalysisSourceContext::github("acme/", Revision::Unknown).is_err());
}

#[test]
fn discovers_supported_files_in_stable_root_relative_order() {
    let root = unique_temp_dir("discovery");
    fs::create_dir_all(root.join("z/nested")).unwrap();
    fs::write(root.join("z/nested/b.json"), "{}").unwrap();
    fs::write(root.join("a.rs"), "fn a() {}").unwrap();
    fs::write(root.join("ignore.txt"), "ignored").unwrap();

    let discovered = discover_sources(&root).unwrap();
    assert_eq!(
        discovered
            .iter()
            .map(|item| (item.relative_path.clone(), item.kind))
            .collect::<Vec<_>>(),
        vec![
            (PathBuf::from("a.rs"), SourceFileKind::Rust),
            (PathBuf::from("z/nested/b.json"), SourceFileKind::Json),
        ]
    );

    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn does_not_follow_symlink_escape() {
    use std::os::unix::fs::symlink;

    let root = unique_temp_dir("symlink-root");
    let outside = unique_temp_dir("symlink-outside");
    fs::write(outside.join("secret.rs"), "fn secret() {}").unwrap();
    symlink(&outside, root.join("escape")).unwrap();

    assert!(discover_sources(&root).unwrap().is_empty());

    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(outside).unwrap();
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("chirograph-{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}
