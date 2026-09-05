use chirograph_core::model::{Revision, SourceId};

use crate::AnalysisError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisSourceContext {
    pub repository: String,
    pub source: SourceId,
    pub namespace: String,
    pub revision: Revision,
}

impl AnalysisSourceContext {
    pub fn github(repository: &str, revision: Revision) -> Result<Self, AnalysisError> {
        let repository = repository.trim();
        let mut parts = repository.split('/');
        let owner = parts.next().unwrap_or_default();
        let name = parts.next().unwrap_or_default();
        if parts.next().is_some()
            || !valid_component(owner)
            || !valid_component(name)
            || owner.is_empty()
            || name.is_empty()
        {
            return Err(AnalysisError::InvalidRepository(repository.to_owned()));
        }
        let source = SourceId::new(format!("github:{repository}"))
            .map_err(|_| AnalysisError::InvalidRepository(repository.to_owned()))?;
        Ok(Self {
            repository: repository.to_owned(),
            source,
            namespace: name.to_owned(),
            revision,
        })
    }
}

fn valid_component(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
}
