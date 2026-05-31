pub mod github;
pub mod gitlab;
pub mod local_git;

use crate::parse::ParsedDocument;

/// Errors from source fetching.
#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    #[error("fetch failed: {0}")]
    FetchFailed(String),
    #[error("parse failed: {0}")]
    ParseFailed(#[from] crate::parse::ParseError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// A Source fetches data from an external system and yields parsed
/// documents ready for ingestion.
pub trait Source {
    fn fetch(&self) -> Result<Vec<ParsedDocument>, SourceError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::{ParsedDocument, ParsedEntity};

    struct FakeSource;

    impl Source for FakeSource {
        fn fetch(&self) -> Result<Vec<ParsedDocument>, SourceError> {
            Ok(vec![ParsedDocument {
                doc_id: "fake".to_string(),
                entities: vec![ParsedEntity {
                    name: "test".to_string(),
                    entity_type: "fake".to_string(),
                    supporting_text: None,
                }],
                relations: vec![],
            }])
        }
    }

    #[test]
    fn fake_source_returns_documents() {
        let docs = FakeSource.fetch().unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].doc_id, "fake");
    }
}
