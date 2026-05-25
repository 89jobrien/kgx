use super::{ParseError, ParsedDocument, ParsedEntity, ParsedRelation, Parser};

/// Parses Graphviz DOT format into entities and relations.
///
/// Supports a subset of DOT:
/// - Node declarations: `nodename;` or `nodename [label="Label"];`
/// - Edges: `A -> B;` or `A -> B [label="rel"];`
/// - Undirected edges: `A -- B;`
/// - `digraph` and `graph` wrappers (stripped)
///
/// Node names become entity names. Edge labels become relation types.
pub struct DotParser;

impl Parser for DotParser {
    fn parse(&self, input: &str, doc_id: &str) -> Result<ParsedDocument, ParseError> {
        let mut entities = Vec::new();
        let mut relations = Vec::new();
        let mut seen_entities = std::collections::HashSet::new();

        // Normalize: split on `;` to handle multiple statements per line,
        // and strip `{ }` wrappers.
        let stripped = input.replace(['{', '}'], "\n");

        for raw_line in stripped.lines() {
            for stmt in raw_line.split(';') {
                let trimmed = stmt.trim();

                if trimmed.is_empty()
                    || trimmed.starts_with("digraph")
                    || trimmed.starts_with("graph")
                    || trimmed.starts_with("strict")
                    || trimmed.starts_with("//")
                {
                    continue;
                }

                if trimmed.contains("->") || trimmed.contains("--") {
                    parse_edge(trimmed, &mut entities, &mut relations, &mut seen_entities);
                } else {
                    parse_node(trimmed, &mut entities, &mut seen_entities);
                }
            }
        }

        Ok(ParsedDocument {
            doc_id: doc_id.to_string(),
            entities,
            relations,
        })
    }
}

fn extract_attr(s: &str, key: &str) -> Option<String> {
    let attr_start = s.find('[')?;
    let attr_end = s.rfind(']')?;
    let attrs = &s[attr_start + 1..attr_end];

    for part in attrs.split(',') {
        let part = part.trim();
        if let Some(rest) = part.strip_prefix(key) {
            let rest = rest.trim().strip_prefix('=')?;
            let val = rest.trim().trim_matches('"').trim_matches('\'');
            return Some(val.to_string());
        }
    }
    None
}

fn ensure_entity(
    name: &str,
    label: Option<&str>,
    entities: &mut Vec<ParsedEntity>,
    seen: &mut std::collections::HashSet<String>,
) {
    let key = name.to_lowercase();
    if seen.insert(key) {
        entities.push(ParsedEntity {
            name: label.unwrap_or(name).to_string(),
            entity_type: "node".to_string(),
            supporting_text: None,
        });
    }
}

fn parse_node(
    line: &str,
    entities: &mut Vec<ParsedEntity>,
    seen: &mut std::collections::HashSet<String>,
) {
    let name = line
        .split('[')
        .next()
        .unwrap_or(line)
        .trim()
        .trim_matches('"');
    if name.is_empty() {
        return;
    }
    let label = extract_attr(line, "label");
    ensure_entity(name, label.as_deref(), entities, seen);
}

fn parse_edge(
    line: &str,
    entities: &mut Vec<ParsedEntity>,
    relations: &mut Vec<ParsedRelation>,
    seen: &mut std::collections::HashSet<String>,
) {
    let (separator, is_directed) = if line.contains("->") {
        ("->", true)
    } else {
        ("--", false)
    };

    let parts_and_attrs: Vec<&str> = line.splitn(2, '[').collect();
    let node_part = parts_and_attrs[0];
    let rel_label = extract_attr(line, "label");

    let nodes: Vec<&str> = node_part
        .split(separator)
        .map(|s| s.trim().trim_matches('"'))
        .collect();
    if nodes.len() < 2 {
        return;
    }

    let source = nodes[0];
    let target = nodes[1];
    if source.is_empty() || target.is_empty() {
        return;
    }

    ensure_entity(source, None, entities, seen);
    ensure_entity(target, None, entities, seen);

    let rel_type = if is_directed {
        rel_label.as_deref().unwrap_or("directed")
    } else {
        rel_label.as_deref().unwrap_or("undirected")
    };

    relations.push(ParsedRelation {
        source: source.to_string(),
        target: target.to_string(),
        relation_type: rel_type.to_string(),
        confidence: 1.0,
        supporting_text: None,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::conformance;

    #[test]
    fn dot_parser_satisfies_contract() {
        conformance::assert_parser_contract(&DotParser, "dot");
    }

    #[test]
    fn simple_digraph() {
        let dot = r#"
digraph G {
    A -> B;
    B -> C;
}
"#;
        let doc = DotParser.parse(dot, "d1").unwrap();
        assert_eq!(doc.entities.len(), 3);
        assert_eq!(doc.relations.len(), 2);
        assert_eq!(doc.relations[0].source, "A");
        assert_eq!(doc.relations[0].target, "B");
        assert_eq!(doc.relations[0].relation_type, "directed");
    }

    #[test]
    fn undirected_graph() {
        let dot = "graph G {\n    A -- B;\n}\n";
        let doc = DotParser.parse(dot, "d1").unwrap();
        assert_eq!(doc.relations.len(), 1);
        assert_eq!(doc.relations[0].relation_type, "undirected");
    }

    #[test]
    fn edge_with_label() {
        let dot = "digraph { A -> B [label=\"uses\"]; }";
        let doc = DotParser.parse(dot, "d1").unwrap();
        assert_eq!(doc.relations[0].relation_type, "uses");
    }

    #[test]
    fn node_declarations() {
        let dot = "digraph {\n    A [label=\"Alpha\"];\n    B;\n    A -> B;\n}\n";
        let doc = DotParser.parse(dot, "d1").unwrap();
        let names: Vec<&str> = doc.entities.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"Alpha"));
        assert!(names.contains(&"B"));
        assert_eq!(doc.relations.len(), 1);
    }

    #[test]
    fn empty_input() {
        let doc = DotParser.parse("", "d1").unwrap();
        assert!(doc.entities.is_empty());
    }

    #[test]
    fn deduplicates_nodes_from_edges() {
        let dot = "digraph { A -> B; A -> C; B -> C; }";
        let doc = DotParser.parse(dot, "d1").unwrap();
        assert_eq!(doc.entities.len(), 3);
        assert_eq!(doc.relations.len(), 3);
    }
}
