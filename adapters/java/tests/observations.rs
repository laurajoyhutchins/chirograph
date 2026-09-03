use chirograph_core::model::{Revision, SourceId};
use chirograph_java_adapter::{JavaFactKind, observe_java_source};

#[test]
fn converts_tree_sitter_facts_into_exact_revision_chirograph_observations() {
    let source = r#"class ProducerConfig {
    public static final String ENABLE_IDEMPOTENCE_CONFIG = "enable.idempotence";
    void validate(int inFlightConnection) {
        if (MAX_ALLOWED < inFlightConnection) {
            throw new ConfigException("must be at most 5");
        }
    }
}
"#;
    let source_id = SourceId::new("kafka.repo").expect("source id");
    let revision = Revision::Exact("5e3bc31a7dbc354155932e38ab35d11dd71b97bf".into());

    let acquisition = observe_java_source(
        source_id.clone(),
        revision.clone(),
        "ProducerConfig.java",
        source,
    )
    .expect("valid Java acquisition");

    assert!(
        acquisition
            .facts
            .iter()
            .any(|fact| fact.kind == JavaFactKind::ConditionalThrow)
    );
    assert_eq!(acquisition.observations.len(), acquisition.facts.len());
    assert!(acquisition.observations.iter().all(|observation| {
        observation.source == source_id && observation.revision == revision
    }));
    assert!(acquisition.observations.iter().all(|observation| {
        observation.locator.starts_with("ProducerConfig.java:L")
            && observation.fact.contains("Java ")
    }));
}
