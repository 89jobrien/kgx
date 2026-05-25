use super::{ParseError, ParsedDocument, ParsedEntity, ParsedRelation, Parser};

/// Parses YAML into entities and relations.
///
/// Expected format:
/// ```yaml
/// entities:
///   - name: Rust
///     type: language
///     supporting_text: systems programming
/// relations:
///   - source: Rust
///     target: Cargo
///     type: has_tool
///     confidence: 1.0
///     supporting_text: build system
/// ```
pub struct YamlParser;

#[derive(serde::Deserialize, Default)]
struct YamlInput {
    #[serde(default)]
    entities: Vec<YamlEntity>,
    #[serde(default)]
    relations: Vec<YamlRelation>,
}

#[derive(serde::Deserialize)]
struct YamlEntity {
    name: String,
    #[serde(rename = "type", default = "default_entity_type")]
    entity_type: String,
    supporting_text: Option<String>,
}

#[derive(serde::Deserialize)]
struct YamlRelation {
    source: String,
    target: String,
    #[serde(rename = "type", default = "default_relation_type")]
    relation_type: String,
    #[serde(default = "default_confidence")]
    confidence: f64,
    supporting_text: Option<String>,
}

fn default_entity_type() -> String {
    "entity".to_string()
}

fn default_relation_type() -> String {
    "related".to_string()
}

fn default_confidence() -> f64 {
    1.0
}

impl Parser for YamlParser {
    fn parse(&self, input: &str, doc_id: &str) -> Result<ParsedDocument, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParsedDocument {
                doc_id: doc_id.to_string(),
                entities: vec![],
                relations: vec![],
            });
        }

        let data: YamlInput = match serde_yaml::from_str(input) {
            Ok(d) => d,
            Err(_) => {
                // Input is not structured YAML for our schema; return empty.
                return Ok(ParsedDocument {
                    doc_id: doc_id.to_string(),
                    entities: vec![],
                    relations: vec![],
                });
            }
        };

        let entities = data
            .entities
            .into_iter()
            .map(|e| ParsedEntity {
                name: e.name,
                entity_type: e.entity_type,
                supporting_text: e.supporting_text,
            })
            .collect();

        let relations = data
            .relations
            .into_iter()
            .map(|r| ParsedRelation {
                source: r.source,
                target: r.target,
                relation_type: r.relation_type,
                confidence: r.confidence,
                supporting_text: r.supporting_text,
            })
            .collect();

        Ok(ParsedDocument {
            doc_id: doc_id.to_string(),
            entities,
            relations,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::conformance;

    #[test]
    fn yaml_parser_satisfies_contract() {
        conformance::assert_parser_contract(&YamlParser, "yaml");
    }

    #[test]
    fn full_yaml() {
        let yaml = r#"
entities:
  - name: Rust
    type: language
    supporting_text: systems programming
  - name: Go
    type: language
relations:
  - source: Rust
    target: Go
    type: competes_with
    confidence: 0.7
"#;
        let doc = YamlParser.parse(yaml, "d1").unwrap();
        assert_eq!(doc.entities.len(), 2);
        assert_eq!(doc.relations.len(), 1);
        assert_eq!(doc.relations[0].confidence, 0.7);
    }

    #[test]
    fn entities_only_yaml() {
        let yaml = "entities:\n  - name: A\n    type: node\n";
        let doc = YamlParser.parse(yaml, "d1").unwrap();
        assert_eq!(doc.entities.len(), 1);
        assert!(doc.relations.is_empty());
    }

    #[test]
    fn empty_input() {
        let doc = YamlParser.parse("", "d1").unwrap();
        assert!(doc.entities.is_empty());
    }

    #[test]
    fn default_confidence_and_type() {
        let yaml = r#"
entities:
  - name: X
relations:
  - source: X
    target: Y
"#;
        let doc = YamlParser.parse(yaml, "d1").unwrap();
        assert_eq!(doc.entities[0].entity_type, "entity");
        assert_eq!(doc.relations[0].confidence, 1.0);
        assert_eq!(doc.relations[0].relation_type, "related");
    }

    #[test]
    fn invalid_yaml_returns_empty() {
        let doc = YamlParser.parse("{{bad yaml", "d1").unwrap();
        assert!(doc.entities.is_empty());
        assert!(doc.relations.is_empty());
    }
}
