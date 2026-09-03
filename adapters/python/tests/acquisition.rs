use chirograph_core::model::{Revision, SourceId};
use chirograph_python_adapter::{
    PythonAdapterError, PythonFactKind, extract_python_facts, observe_python_source,
};

const SOURCE: &str = r#"\"\"\"Module contract docs.\"\"\"

from typing import Any

class ValidatorModel(BaseModel):
    \"\"\"Validation model docs.\"\"\"
    value: str

    @field_validator(\"value\", mode=\"before\", json_schema_input_type=int | str)
    @classmethod
    def cast_ints(cls, value: Any) -> Any:
        # Normalize integers before validation.
        if isinstance(value, int):
            return str(value)
        raise ValueError(\"unsupported\")

def test_validator():
    result = ValidatorModel(value=\"x\")
    assert result.value == \"x\"
"#;

#[test]
fn extracts_python_contract_relevant_facts_without_framework_knowledge() {
    let facts = extract_python_facts("models.py", SOURCE).expect("valid Python should parse");

    assert!(facts.iter().any(|fact| fact.kind == PythonFactKind::Module));
    assert!(facts.iter().any(|fact| {
        fact.kind == PythonFactKind::ClassDefinition
            && fact.name.as_deref() == Some("ValidatorModel")
    }));
    assert!(facts.iter().any(|fact| {
        fact.kind == PythonFactKind::FunctionDefinition
            && fact.name.as_deref() == Some("cast_ints")
            && fact.annotation.as_deref() == Some("Any")
    }));
    assert!(facts.iter().any(|fact| {
        fact.kind == PythonFactKind::AnnotatedAssignment
            && fact.name.as_deref() == Some("value")
            && fact.annotation.as_deref() == Some("str")
    }));
    assert!(facts.iter().any(|fact| {
        fact.kind == PythonFactKind::Decorator
            && fact.name.as_deref() == Some("field_validator")
            && fact.text.contains("json_schema_input_type=int | str")
    }));
    assert!(facts.iter().any(|fact| {
        fact.kind == PythonFactKind::Conditional
            && fact.condition.as_deref() == Some("isinstance(value, int)")
    }));
    assert!(facts.iter().any(|fact| fact.kind == PythonFactKind::Return));
    assert!(facts.iter().any(|fact| fact.kind == PythonFactKind::Raise));
    assert!(facts.iter().any(|fact| fact.kind == PythonFactKind::Call));
    assert!(facts.iter().any(|fact| fact.kind == PythonFactKind::Comment));
    assert_eq!(
        facts
            .iter()
            .filter(|fact| fact.kind == PythonFactKind::Docstring)
            .count(),
        2
    );
    assert!(facts.iter().any(|fact| fact.kind == PythonFactKind::TestAssertion));
}

#[test]
fn preserves_exact_byte_and_source_spans() {
    let facts = extract_python_facts("models.py", SOURCE).expect("valid Python should parse");
    let field = facts
        .iter()
        .find(|fact| fact.kind == PythonFactKind::AnnotatedAssignment)
        .expect("annotated assignment should be observed");
    let expected_start = SOURCE.find("value: str").expect("fixture contains field");

    assert_eq!(field.span.start_byte, expected_start);
    assert_eq!(field.span.end_byte, expected_start + "value: str".len());
    assert!(field.span.start_line > 0);
    assert!(field.span.start_column > 0);
    assert_eq!(field.path, "models.py");
}

#[test]
fn emits_core_observations_at_the_supplied_exact_revision() {
    let source_id = SourceId::new("python.fixture").expect("valid source id");
    let revision = Revision::Exact("0123456789abcdef0123456789abcdef01234567".into());
    let acquisition = observe_python_source(
        source_id.clone(),
        revision.clone(),
        "models.py",
        SOURCE,
    )
    .expect("valid Python should be observed");

    assert_eq!(acquisition.facts.len(), acquisition.observations.len());
    assert!(!acquisition.observations.is_empty());
    assert!(acquisition.observations.iter().all(|observation| {
        observation.source == source_id
            && observation.revision == revision
            && observation.locator.starts_with("models.py:B")
            && observation.locator.contains(":L")
    }));
}

#[test]
fn fails_closed_on_syntax_errors() {
    let error = extract_python_facts("broken.py", "def broken(:\n")
        .expect_err("syntax errors must not become partial source facts");

    assert_eq!(error, PythonAdapterError::SyntaxError);
}
