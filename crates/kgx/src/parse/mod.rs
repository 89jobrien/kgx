pub mod csv;
pub mod dot;
pub mod markdown;
pub mod yaml;

/// A parsed entity, ready for ingestion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedEntity {
    pub name: String,
    pub entity_type: String,
    pub supporting_text: Option<String>,
}

/// A parsed relation, ready for ingestion.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedRelation {
    pub source: String,
    pub target: String,
    pub relation_type: String,
    pub confidence: f64,
    pub supporting_text: Option<String>,
}

/// Output of a Parser.
#[derive(Debug, Clone)]
pub struct ParsedDocument {
    pub doc_id: String,
    pub entities: Vec<ParsedEntity>,
    pub relations: Vec<ParsedRelation>,
}

/// Errors from parsing.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("parse failed: {0}")]
    Failed(String),
}

/// Converts a source format into entities and relations.
pub trait Parser {
    fn parse(&self, input: &str, doc_id: &str) -> Result<ParsedDocument, ParseError>;
}

/// Conformance test suite for the Parser trait.
/// Any implementation must:
/// 1. Return Ok for empty input (never panic)
/// 2. Set doc_id on the returned ParsedDocument to match the input
/// 3. Every relation source must appear as an entity name
/// 4. Confidence values must be in [0.0, 1.0]
#[cfg(test)]
pub mod conformance {
    use super::*;

    pub fn assert_parser_contract(parser: &dyn Parser, label: &str) {
        // 1. Empty input succeeds
        let doc = parser
            .parse("", "empty-doc")
            .unwrap_or_else(|e| panic!("{label}: empty input must not error: {e}"));
        assert_eq!(doc.doc_id, "empty-doc", "{label}: doc_id must match input");

        // 2. doc_id propagation with non-empty input
        let doc = parser
            .parse("# Hello\n\nSome content.\n", "test-doc")
            .unwrap_or_else(|e| panic!("{label}: simple input must not error: {e}"));
        assert_eq!(doc.doc_id, "test-doc", "{label}: doc_id must match input");

        // 3. Every relation source must name an entity
        let entity_names: std::collections::HashSet<&str> =
            doc.entities.iter().map(|e| e.name.as_str()).collect();
        for rel in &doc.relations {
            assert!(
                entity_names.contains(rel.source.as_str()),
                "{label}: relation source '{}' not found in entities",
                rel.source
            );
        }

        // 4. Confidence in range
        for rel in &doc.relations {
            assert!(
                (0.0..=1.0).contains(&rel.confidence),
                "{label}: confidence {} out of [0.0, 1.0]",
                rel.confidence
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoParser;

    impl Parser for EchoParser {
        fn parse(&self, _input: &str, doc_id: &str) -> Result<ParsedDocument, ParseError> {
            Ok(ParsedDocument {
                doc_id: doc_id.to_string(),
                entities: vec![ParsedEntity {
                    name: "test".to_string(),
                    entity_type: "echo".to_string(),
                    supporting_text: None,
                }],
                relations: vec![],
            })
        }
    }

    #[test]
    fn echo_parser_returns_parsed_document() {
        let p = EchoParser;
        let doc = p.parse("anything", "d1").unwrap();
        assert_eq!(doc.doc_id, "d1");
        assert_eq!(doc.entities.len(), 1);
        assert_eq!(doc.entities[0].name, "test");
        assert!(doc.relations.is_empty());
    }

    #[test]
    fn echo_parser_satisfies_contract() {
        conformance::assert_parser_contract(&EchoParser, "echo");
    }
}
