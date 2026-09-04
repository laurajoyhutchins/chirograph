use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use chirograph_benchmark::model::{
    BenchmarkCase, FixtureFileV1, GOLDEN_SCHEMA, GoldenV1, SPECIMEN_SCHEMA, SpecimenV1, UpstreamV1,
    parse_specimen_yaml,
};
use chirograph_benchmark::source::{SourceError, SourceFetcher, refresh_sources, verify_sources};
use sha2::{Digest, Sha256};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
const OLD_REVISION: &str = "0123456789abcdef0123456789abcdef01234567";
const NEW_REVISION: &str = "89abcdef0123456789abcdef0123456789abcdef";

#[derive(Default)]
struct FakeFetcher {
    files: BTreeMap<(String, String, String), Vec<u8>>,
}

impl FakeFetcher {
    fn with(mut self, repository: &str, revision: &str, path: &str, bytes: &[u8]) -> Self {
        self.files.insert(
            (repository.to_owned(), revision.to_owned(), path.to_owned()),
            bytes.to_vec(),
        );
        self
    }
}

impl SourceFetcher for FakeFetcher {
    fn fetch(&self, repository: &str, revision: &str, path: &str) -> Result<Vec<u8>, SourceError> {
        self.files
            .get(&(repository.to_owned(), revision.to_owned(), path.to_owned()))
            .cloned()
            .ok_or_else(|| SourceError::Git("fake source missing".to_owned()))
    }
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn temp_root() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "chirograph-benchmark-source-{}-{}",
        std::process::id(),
        NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(root.join("fixture/src")).expect("create fixture dir");
    root
}

fn golden() -> GoldenV1 {
    GoldenV1 {
        schema: GOLDEN_SCHEMA.to_owned(),
        contracts: Vec::new(),
        representations: Vec::new(),
        authority_claims: Vec::new(),
        relationships: Vec::new(),
        clauses: Vec::new(),
        lifecycle: Vec::new(),
        expected_findings: Vec::new(),
        non_contracts: Vec::new(),
    }
}

fn benchmark_case(root: &Path, bytes: &[u8]) -> BenchmarkCase {
    let fixture_path = root.join("fixture/src/lib.rs");
    fs::write(&fixture_path, bytes).expect("write fixture");
    let golden_path = root.join("golden.yaml");
    fs::write(&golden_path, b"golden-bytes-must-not-change\n").expect("write golden sentinel");
    let specimen = SpecimenV1 {
        schema: SPECIMEN_SCHEMA.to_owned(),
        id: "repo/scenario/case".to_owned(),
        repository: "repo".to_owned(),
        scenario: "scenario".to_owned(),
        upstream: UpstreamV1 {
            repository: "owner/repo".to_owned(),
            revision: OLD_REVISION.to_owned(),
        },
        files: vec![FixtureFileV1 {
            fixture_path: "fixture/src/lib.rs".to_owned(),
            upstream_path: "src/lib.rs".to_owned(),
            sha256: sha256(bytes),
        }],
    };
    fs::write(root.join("specimen.yaml"), b"original-specimen-bytes\n")
        .expect("write specimen sentinel");

    BenchmarkCase {
        id: specimen.id.clone(),
        repository: specimen.repository.clone(),
        scenario: specimen.scenario.clone(),
        root: root.to_path_buf(),
        fixture_dir: root.join("fixture"),
        specimen_path: root.join("specimen.yaml"),
        golden_path,
        specimen,
        golden: golden(),
    }
}

#[test]
fn verify_is_read_only_and_rejects_remote_mismatch() {
    let root = temp_root();
    let case = benchmark_case(&root, b"same bytes\n");
    let fixture_before = fs::read(root.join("fixture/src/lib.rs")).expect("fixture before");
    let specimen_before = fs::read(&case.specimen_path).expect("specimen before");
    let golden_before = fs::read(&case.golden_path).expect("golden before");

    let matching =
        FakeFetcher::default().with("owner/repo", OLD_REVISION, "src/lib.rs", b"same bytes\n");
    verify_sources(std::slice::from_ref(&case), &matching).expect("matching source verifies");
    assert_eq!(
        fs::read(root.join("fixture/src/lib.rs")).unwrap(),
        fixture_before
    );
    assert_eq!(fs::read(&case.specimen_path).unwrap(), specimen_before);
    assert_eq!(fs::read(&case.golden_path).unwrap(), golden_before);

    let mismatching = FakeFetcher::default().with(
        "owner/repo",
        OLD_REVISION,
        "src/lib.rs",
        b"different bytes\n",
    );
    assert!(matches!(
        verify_sources(std::slice::from_ref(&case), &mismatching),
        Err(SourceError::RemoteMismatch(_))
    ));

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn refresh_updates_only_fixture_and_specimen_provenance() {
    let root = temp_root();
    let mut case = benchmark_case(&root, b"old bytes\n");
    let golden_before = fs::read(&case.golden_path).expect("golden before");
    let fetcher = FakeFetcher::default().with(
        "owner/repo",
        NEW_REVISION,
        "src/lib.rs",
        b"new upstream bytes\n",
    );

    refresh_sources(std::slice::from_mut(&mut case), NEW_REVISION, &fetcher)
        .expect("refresh exact revision");

    let refreshed = fs::read(root.join("fixture/src/lib.rs")).expect("refreshed fixture");
    assert_eq!(refreshed, b"new upstream bytes\n");
    assert_eq!(case.specimen.upstream.revision, NEW_REVISION);
    assert_eq!(case.specimen.files[0].sha256, sha256(&refreshed));
    let specimen_text = fs::read_to_string(&case.specimen_path).expect("refreshed specimen");
    let parsed = parse_specimen_yaml(&specimen_text).expect("refreshed specimen parses");
    assert_eq!(parsed.upstream.revision, NEW_REVISION);
    assert_eq!(parsed.files[0].sha256, sha256(&refreshed));
    assert_eq!(fs::read(&case.golden_path).unwrap(), golden_before);

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn refresh_rejects_symbolic_revision_before_mutating() {
    let root = temp_root();
    let mut case = benchmark_case(&root, b"old bytes\n");
    let fixture_before = fs::read(root.join("fixture/src/lib.rs")).expect("fixture before");
    let specimen_before = fs::read(&case.specimen_path).expect("specimen before");
    let golden_before = fs::read(&case.golden_path).expect("golden before");

    let error = refresh_sources(
        std::slice::from_mut(&mut case),
        "main",
        &FakeFetcher::default(),
    )
    .expect_err("symbolic revision must fail");
    assert!(matches!(error, SourceError::InvalidRevision(_)));
    assert_eq!(
        fs::read(root.join("fixture/src/lib.rs")).unwrap(),
        fixture_before
    );
    assert_eq!(fs::read(&case.specimen_path).unwrap(), specimen_before);
    assert_eq!(fs::read(&case.golden_path).unwrap(), golden_before);

    fs::remove_dir_all(root).expect("remove temp root");
}
