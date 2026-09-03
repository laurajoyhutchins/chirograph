use chirograph_core::model::{Revision, SourceId};
use chirograph_java_adapter::{JavaFactKind, observe_java_source, rank_java_evidence};

const SOURCE: &str = r#"
class ProducerConfig {
    static final String ENABLE_IDEMPOTENCE_DOC =
        "Idempotence requires max.in.flight.requests.per.connection to be at most 5.";
    static final String BATCH_DOC = "Compression batches records before sending.";

    void validate(boolean idempotenceEnabled, int maxInFlight, int retries) {
        if (idempotenceEnabled && maxInFlight > 5) {
            throw new ConfigException("max in flight must be at most 5");
        }
        if (retries < 0) {
            throw new ConfigException("retries must be non-negative");
        }
    }
}
"#;

fn acquisition() -> chirograph_java_adapter::JavaAcquisition {
    observe_java_source(
        SourceId::new("example.repo").unwrap(),
        Revision::Exact("0123456789abcdef0123456789abcdef01234567".into()),
        "src/ProducerConfig.java",
        SOURCE,
    )
    .unwrap()
}

#[test]
fn ranks_semantically_related_facts_ahead_of_java_noise() {
    let acquisition = acquisition();
    let candidates = rank_java_evidence(
        &acquisition,
        "idempotence max.in.flight",
        &[
            JavaFactKind::FieldDeclaration,
            JavaFactKind::ConditionalThrow,
        ],
    );

    assert_eq!(candidates.len(), 4);
    assert_eq!(candidates[0].fact.kind, JavaFactKind::ConditionalThrow);
    assert_eq!(
        candidates[0].matched_terms,
        ["flight", "idempotence", "max"]
    );
    assert_eq!(candidates[1].fact.kind, JavaFactKind::FieldDeclaration);
    assert_eq!(
        candidates[1].matched_terms,
        ["flight", "idempotence", "max"]
    );
    assert!(candidates[0].score > candidates[2].score);
    assert!(candidates[1].score > candidates[2].score);
}

#[test]
fn normalizes_dotted_snake_and_camel_case_terms_without_substring_matches() {
    let acquisition = acquisition();
    let candidates = rank_java_evidence(
        &acquisition,
        "idempotence max.in.flight",
        &[JavaFactKind::ConditionalThrow],
    );

    let relevant = &candidates[0];
    assert!(relevant.fact.text.contains("idempotenceEnabled"));
    assert_eq!(relevant.matched_terms, ["flight", "idempotence", "max"]);

    let retries = candidates
        .iter()
        .find(|candidate| candidate.fact.text.contains("retries < 0"))
        .unwrap();
    assert!(retries.matched_terms.is_empty());
    assert_eq!(retries.score, 0);
}

#[test]
fn candidate_observations_preserve_exact_revision_and_locator() {
    let acquisition = acquisition();
    let candidates = rank_java_evidence(
        &acquisition,
        "idempotence max.in.flight",
        &[JavaFactKind::ConditionalThrow],
    );

    let candidate = &candidates[0];
    assert_eq!(
        candidate.observation.revision,
        acquisition.observations[candidate.fact_index].revision
    );
    assert_eq!(
        candidate.observation.locator,
        acquisition.observations[candidate.fact_index].locator
    );
    assert_eq!(
        candidate.observation.id,
        acquisition.observations[candidate.fact_index].id
    );
}
