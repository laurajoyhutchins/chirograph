use chirograph_java_adapter::{JavaFactKind, extract_java_facts};

#[test]
fn extracts_field_declarations_with_exact_source_ranges() {
    let source = r#"class ProducerConfig {
    public static final String ENABLE_IDEMPOTENCE_CONFIG = "enable.idempotence";
}
"#;

    let facts = extract_java_facts("ProducerConfig.java", source).expect("valid Java");
    let field = facts
        .iter()
        .find(|fact| fact.kind == JavaFactKind::FieldDeclaration)
        .expect("field declaration fact");

    assert_eq!(field.name.as_deref(), Some("ENABLE_IDEMPOTENCE_CONFIG"));
    assert_eq!(field.path, "ProducerConfig.java");
    assert_eq!(field.span.start_line, 2);
    assert_eq!(field.span.end_line, 2);
    assert!(field.text.contains("enable.idempotence"));
}

#[test]
fn extracts_method_invocations_without_framework_knowledge() {
    let source = r#"class ProducerConfig {
    void build() {
        new ConfigDef().define(ENABLE_IDEMPOTENCE_CONFIG, Type.BOOLEAN, true, Importance.LOW, ENABLE_IDEMPOTENCE_DOC);
    }
}
"#;

    let facts = extract_java_facts("ProducerConfig.java", source).expect("valid Java");
    let define = facts
        .iter()
        .find(|fact| {
            fact.kind == JavaFactKind::MethodInvocation && fact.name.as_deref() == Some("define")
        })
        .expect("define invocation fact");

    assert!(define.text.contains("ENABLE_IDEMPOTENCE_CONFIG"));
    assert!(define.text.contains("true"));
}

#[test]
fn collapses_if_with_throw_into_a_conditional_throw_fact() {
    let source = r#"class ProducerConfig {
    void validate(int inFlightConnection) {
        if (MAX_IN_FLIGHT_REQUESTS_PER_CONNECTION_FOR_IDEMPOTENCE < inFlightConnection) {
            throw new ConfigException("must be set to at most 5");
        }
    }
}
"#;

    let facts = extract_java_facts("ProducerConfig.java", source).expect("valid Java");
    let failure = facts
        .iter()
        .find(|fact| fact.kind == JavaFactKind::ConditionalThrow)
        .expect("conditional throw fact");

    assert_eq!(failure.name.as_deref(), Some("ConfigException"));
    assert!(
        failure
            .condition
            .as_deref()
            .expect("condition")
            .contains("MAX_IN_FLIGHT_REQUESTS_PER_CONNECTION_FOR_IDEMPOTENCE < inFlightConnection")
    );
    assert!(failure.text.contains("must be set to at most 5"));
}

#[test]
fn classifies_assertion_calls_as_test_assertions() {
    let source = r#"class KafkaProducerTest {
    void rejectsInvalidConfiguration() {
        assertThrows(ConfigException.class, () -> new ProducerConfig(props));
    }
}
"#;

    let facts = extract_java_facts("KafkaProducerTest.java", source).expect("valid Java");
    let assertion = facts
        .iter()
        .find(|fact| fact.kind == JavaFactKind::TestAssertion)
        .expect("test assertion fact");

    assert_eq!(assertion.name.as_deref(), Some("assertThrows"));
    assert!(assertion.text.contains("ConfigException.class"));
}
