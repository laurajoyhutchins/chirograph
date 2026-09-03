use chirograph_core::model::{Revision, SourceId};
use chirograph_java_adapter::{
    JavaEvidenceCandidate, JavaFactKind, observe_java_source, rank_java_evidence,
};
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
    let source =
        fs::read_to_string(&path).map_err(|error| format!("cannot read {path}: {error}"))?;
    let source_id =
        SourceId::new("kafka.repo").map_err(|error| format!("invalid source id: {error:?}"))?;
    let acquisition = observe_java_source(
        source_id,
        Revision::Exact(KAFKA_REVISION.into()),
        PRODUCER_CONFIG_PATH,
        &source,
    )
    .map_err(|error| error.to_string())?;

    let documentation_candidates = rank_java_evidence(
        &acquisition,
        "idempotence conflicting configurations explicitly enabled disabled exception",
        &[JavaFactKind::FieldDeclaration],
    );
    let validator_candidates = rank_java_evidence(
        &acquisition,
        "idempotent producer max flight at most current value exception",
        &[JavaFactKind::ConditionalThrow],
    );

    let doc = best_candidate(
        &documentation_candidates,
        4,
        "Kafka idempotence documentation evidence",
    )?;
    let validator = best_candidate(
        &validator_candidates,
        4,
        "Kafka max-in-flight validator evidence",
    )?;
    let doc = &doc.observation;
    let validator = &validator.observation;

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

fn best_candidate<'a>(
    candidates: &'a [JavaEvidenceCandidate],
    minimum_matches: usize,
    description: &str,
) -> Result<&'a JavaEvidenceCandidate, String> {
    let candidate = candidates
        .first()
        .ok_or_else(|| format!("no {description} candidates were observed"))?;
    if candidate.matched_terms.len() < minimum_matches {
        return Err(format!(
            "top {description} candidate matched only {} semantic terms: {:?}",
            candidate.matched_terms.len(),
            candidate.matched_terms
        ));
    }
    Ok(candidate)
}
