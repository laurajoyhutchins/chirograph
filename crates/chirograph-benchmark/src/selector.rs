use crate::model::BenchmarkCase;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectorError {
    message: String,
}

impl SelectorError {
    fn invalid(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for SelectorError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for SelectorError {}

pub fn select_cases<'a>(
    cases: &'a [BenchmarkCase],
    selector: &str,
) -> Result<Vec<&'a BenchmarkCase>, SelectorError> {
    let matches = if selector == "all" {
        cases.iter().collect::<Vec<_>>()
    } else if let Some(scenario) = selector.strip_prefix("scenario:") {
        if !is_token(scenario) {
            return Err(SelectorError::invalid("invalid scenario selector"));
        }
        cases
            .iter()
            .filter(|case| case.scenario == scenario)
            .collect::<Vec<_>>()
    } else {
        let parts = selector.split('/').collect::<Vec<_>>();
        if parts.iter().any(|part| !is_token(part)) {
            return Err(SelectorError::invalid("invalid benchmark selector"));
        }
        match parts.as_slice() {
            [repository] => cases
                .iter()
                .filter(|case| case.repository == *repository)
                .collect::<Vec<_>>(),
            [repository, scenario] => cases
                .iter()
                .filter(|case| case.repository == *repository && case.scenario == *scenario)
                .collect::<Vec<_>>(),
            [repository, scenario, case_name] => {
                let id = format!("{repository}/{scenario}/{case_name}");
                cases
                    .iter()
                    .filter(|case| case.id == id)
                    .collect::<Vec<_>>()
            }
            _ => return Err(SelectorError::invalid("invalid benchmark selector")),
        }
    };

    if matches.is_empty() {
        return Err(SelectorError::invalid(format!(
            "selector matched no benchmark cases: {selector}"
        )));
    }

    let mut matches = matches;
    matches.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(matches)
}

fn is_token(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}
