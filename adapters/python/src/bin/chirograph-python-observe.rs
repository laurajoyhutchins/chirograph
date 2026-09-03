#![forbid(unsafe_code)]

use std::env;
use std::fs;
use std::process::ExitCode;

use chirograph_core::model::{Revision, SourceId};
use chirograph_python_adapter::{PythonAcquisition, PythonFact, observe_python_source};
use serde::Serialize;

const HELP: &str = "Usage: chirograph-python-observe <source-id> <exact-revision> <path>\n";

#[derive(Debug, Serialize)]
struct SourceDocument {
    id: String,
    kind: &'static str,
    locator: String,
}

#[derive(Debug, Serialize)]
struct ExactRevisionDocument {
    kind: &'static str,
    value: String,
}

#[derive(Debug, Serialize)]
struct ObservationDocument {
    id: String,
    source: String,
    revision: ExactRevisionDocument,
    locator: String,
    fact: String,
}

#[derive(Debug, Serialize)]
struct AcquisitionDocument {
    schema: &'static str,
    source: SourceDocument,
    facts: Vec<PythonFact>,
    observations: Vec<ObservationDocument>,
}

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(output) => {
            print!("{output}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Vec<String>) -> Result<String, String> {
    let [source_id_value, exact_revision, path] = args.as_slice() else {
        return Err(HELP.trim_end().into());
    };
    if exact_revision.is_empty() || exact_revision.trim() != exact_revision {
        return Err("exact revision must be non-empty and trimmed".into());
    }

    let source = fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))?;
    let source_id = SourceId::new(source_id_value.clone())
        .map_err(|error| format!("invalid source id: {error:?}"))?;
    let PythonAcquisition {
        facts,
        observations,
    } = observe_python_source(
        source_id,
        Revision::Exact(exact_revision.clone()),
        path,
        &source,
    )
    .map_err(|error| format!("cannot observe Python source: {error}"))?;

    let document = AcquisitionDocument {
        schema: "chirograph-python-acquisition-v1",
        source: SourceDocument {
            id: source_id_value.clone(),
            kind: "repository",
            locator: path.clone(),
        },
        facts,
        observations: observations
            .into_iter()
            .map(|observation| ObservationDocument {
                id: observation.id.as_str().into(),
                source: observation.source.as_str().into(),
                revision: ExactRevisionDocument {
                    kind: "exact",
                    value: exact_revision.clone(),
                },
                locator: observation.locator,
                fact: observation.fact,
            })
            .collect(),
    };

    serde_json::to_string_pretty(&document)
        .map(|mut json| {
            json.push('\n');
            json
        })
        .map_err(|error| format!("cannot serialize Python acquisition: {error}"))
}
