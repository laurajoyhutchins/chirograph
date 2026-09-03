use chirograph_core::model::{Revision, SourceId};
use chirograph_java_adapter::{JavaFactKind, observe_java_source};
use serde_json::json;
use std::env;
use std::fs;
use std::process::ExitCode;

const KAFKA_REVISION: &str = "5e3bc31a7dbc354155932e38ab35d11dd71b97bf";
const PRODUCER_CONFIG_PATH: &str =
    "clients/src/main/java/org/apache/kafka/clients/producer/ProducerConfig.java";

fn main() -> ExitCode {
    match run() {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, String> {
    let path = env::args()
        .nth(1)
        .ok_or_else(|| "usage: kafka_idempotence <ProducerConfig.java>".to_owned())?;
    let source = fs::read_to_string(&path).map_err(|error| format!("cannot read {path}: {error}"))?;
    let source_id = SourceId::new("kafka.repo").map_err(|error| format!("invalid source id: {error:?}"))?;
    let acquisition = observe_java_source(
        source_id,
        Revision::Exact(KAFKA_REVISION.into()),
        PRODUCER_CONFIG_PATH,
        &source,
    )
    .map_err(|error| error.to_string())?;

    let doc_index = acquisition
        .facts
        .iter()
        .position(|fact| {
            fact.kind == JavaFactKind::FieldDeclaration
                && fact.name.as_deref() == Some("ENABLE_IDEMPOTENCE_DOC")
                && fact.text.contains("not explicitly enabled")
                && fact.text.contains("ConfigException")
        })
        .ok_or_else(|| "Kafka idempotence documentation declaration was not observed".to_owned())?;
    let validator_index = acquisition
        .facts
        .iter()
        .position(|fact| {
            fact.kind == JavaFactKind::ConditionalThrow
                && fact.name.as_deref() == Some("ConfigException")
                && fact
                    .condition
                    .as_deref()
                    .is_some_and(|condition| {
                        condition.contains("MAX_IN_FLIGHT_REQUESTS_PER_CONNECTION_FOR_IDEMPOTENCE")
                            && condition.contains("inFlightConnection")
                    })
        })
        .ok_or_else(|| "Kafka max-in-flight ConfigException guard was not observed".to_owned())?;

    let doc = &acquisition.observations[doc_index];
    let validator = &acquisition.observations[validator_index];
    let evidence = json!({
        "schema": "chirograph-evidence-v1",
        "sources": [
            {
                "id": "kafka.repo",
                "kind": "repository",
                "locator": "https://github.com/apache/kafka"
            }
        ],
        "contracts": [
            {
                "id": "kafka.producer.idempotence",
                "name": "Kafka producer idempotence configuration",
                "facets": ["failure"]
            }
        ],
        "representations": [
            {
                "id": "kafka.producer.idempotence.doc",
                "contract": "kafka.producer.idempotence",
                "source": "kafka.repo",
                "kind": "documentation",
                "locator": doc.locator,
                "facets": ["failure"]
            },
            {
                "id": "kafka.producer.idempotence.validator",
                "contract": "kafka.producer.idempotence",
                "source": "kafka.repo",
                "kind": "validator",
                "locator": validator.locator,
                "facets": ["failure"]
            }
        ],
        "observations": [
            {
                "id": doc.id.as_str(),
                "source": "kafka.repo",
                "revision": {"kind": "exact", "value": KAFKA_REVISION},
                "locator": doc.locator,
                "fact": doc.fact
            },
            {
                "id": validator.id.as_str(),
                "source": "kafka.repo",
                "revision": {"kind": "exact", "value": KAFKA_REVISION},
                "locator": validator.locator,
                "fact": validator.fact
            }
        ],
        "clauses": [
            {
                "id": "clause.implicit-max-in-flight-fallback",
                "contract": "kafka.producer.idempotence",
                "facet": "failure",
                "kind": "guarantee",
                "statement": "When enable.idempotence is not explicitly configured and max.in.flight.requests.per.connection exceeds 5, producer configuration disables idempotence instead of failing construction."
            }
        ],
        "clause_assertions": [
            {
                "clause": "clause.implicit-max-in-flight-fallback",
                "representation": "kafka.producer.idempotence.doc",
                "stance": "supports",
                "evidence": [doc.id.as_str()]
            },
            {
                "clause": "clause.implicit-max-in-flight-fallback",
                "representation": "kafka.producer.idempotence.validator",
                "stance": "contradicts",
                "evidence": [validator.id.as_str()]
            }
        ],
        "relations": [],
        "authority_claims": [
            {
                "contract": "kafka.producer.idempotence",
                "representation": "kafka.producer.idempotence.validator",
                "facet": "failure",
                "basis": "mechanical_enforcement",
                "evidence": [validator.id.as_str()]
            }
        ]
    });

    serde_json::to_string_pretty(&evidence).map_err(|error| error.to_string())
}
