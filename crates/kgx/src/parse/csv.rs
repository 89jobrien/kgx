use super::{ParseError, ParsedDocument, ParsedEntity, ParsedRelation, Parser};

/// Parses CSV into entities and relations.
///
/// Format: two sections separated by a blank line.
/// First section: entity rows with header `name,type,supporting_text`
/// Second section: relation rows with header `source,target,type,confidence,supporting_text`
///
/// If only one section is present, it is detected by header columns.
/// The `supporting_text` column is optional in both sections.
pub struct CsvParser;

impl Parser for CsvParser {
    fn parse(&self, input: &str, doc_id: &str) -> Result<ParsedDocument, ParseError> {
        let mut entities = Vec::new();
        let mut relations = Vec::new();

        if input.trim().is_empty() {
            return Ok(ParsedDocument {
                doc_id: doc_id.to_string(),
                entities,
                relations,
            });
        }

        let sections: Vec<&str> = input.split("\n\n").collect();

        for section in sections {
            let lines: Vec<&str> = section.lines().filter(|l| !l.trim().is_empty()).collect();
            if lines.is_empty() {
                continue;
            }
            let header = lines[0].to_lowercase();
            let cols: Vec<&str> = header.split(',').map(|c| c.trim()).collect();

            if cols.contains(&"source") && cols.contains(&"target") {
                parse_relation_rows(&lines, &cols, &mut relations)?;
            } else if cols.contains(&"name") {
                parse_entity_rows(&lines, &cols, &mut entities)?;
            }
        }

        Ok(ParsedDocument {
            doc_id: doc_id.to_string(),
            entities,
            relations,
        })
    }
}

fn col_index(cols: &[&str], name: &str) -> Option<usize> {
    cols.iter().position(|c| *c == name)
}

fn get_field<'a>(fields: &[&'a str], idx: Option<usize>) -> Option<&'a str> {
    idx.and_then(|i| fields.get(i))
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
}

fn parse_entity_rows(
    lines: &[&str],
    cols: &[&str],
    entities: &mut Vec<ParsedEntity>,
) -> Result<(), ParseError> {
    let name_idx = col_index(cols, "name");
    let type_idx = col_index(cols, "type");
    let text_idx = col_index(cols, "supporting_text");

    for line in &lines[1..] {
        let fields: Vec<&str> = line.split(',').collect();
        let name = get_field(&fields, name_idx).unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        entities.push(ParsedEntity {
            name: name.to_string(),
            entity_type: get_field(&fields, type_idx).unwrap_or("entity").to_string(),
            supporting_text: get_field(&fields, text_idx).map(String::from),
        });
    }
    Ok(())
}

fn parse_relation_rows(
    lines: &[&str],
    cols: &[&str],
    relations: &mut Vec<ParsedRelation>,
) -> Result<(), ParseError> {
    let src_idx = col_index(cols, "source");
    let tgt_idx = col_index(cols, "target");
    let type_idx = col_index(cols, "type");
    let conf_idx = col_index(cols, "confidence");
    let text_idx = col_index(cols, "supporting_text");

    for line in &lines[1..] {
        let fields: Vec<&str> = line.split(',').collect();
        let source = get_field(&fields, src_idx).unwrap_or_default();
        let target = get_field(&fields, tgt_idx).unwrap_or_default();
        if source.is_empty() || target.is_empty() {
            continue;
        }
        let confidence = get_field(&fields, conf_idx)
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(1.0);
        relations.push(ParsedRelation {
            source: source.to_string(),
            target: target.to_string(),
            relation_type: get_field(&fields, type_idx)
                .unwrap_or("related")
                .to_string(),
            confidence,
            supporting_text: get_field(&fields, text_idx).map(String::from),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::conformance;

    #[test]
    fn csv_parser_satisfies_contract() {
        conformance::assert_parser_contract(&CsvParser, "csv");
    }

    #[test]
    fn entities_only() {
        let csv = "name,type,supporting_text\nRust,language,systems programming\nGo,language,\n";
        let doc = CsvParser.parse(csv, "d1").unwrap();
        assert_eq!(doc.entities.len(), 2);
        assert_eq!(doc.entities[0].name, "Rust");
        assert_eq!(doc.entities[0].entity_type, "language");
        assert_eq!(
            doc.entities[0].supporting_text.as_deref(),
            Some("systems programming")
        );
        assert_eq!(doc.entities[1].name, "Go");
        assert!(doc.entities[1].supporting_text.is_none());
        assert!(doc.relations.is_empty());
    }

    #[test]
    fn relations_only() {
        let csv = "source,target,type,confidence\nA,B,uses,0.9\nB,C,extends,1.0\n";
        let doc = CsvParser.parse(csv, "d1").unwrap();
        assert!(doc.entities.is_empty());
        assert_eq!(doc.relations.len(), 2);
        assert_eq!(doc.relations[0].source, "A");
        assert_eq!(doc.relations[0].target, "B");
        assert_eq!(doc.relations[0].confidence, 0.9);
    }

    #[test]
    fn both_sections() {
        let csv = "name,type\nA,node\nB,node\n\nsource,target,type,confidence\nA,B,links,0.8\n";
        let doc = CsvParser.parse(csv, "d1").unwrap();
        assert_eq!(doc.entities.len(), 2);
        assert_eq!(doc.relations.len(), 1);
    }

    #[test]
    fn empty_input() {
        let doc = CsvParser.parse("", "d1").unwrap();
        assert!(doc.entities.is_empty());
        assert!(doc.relations.is_empty());
    }

    #[test]
    fn default_confidence() {
        let csv = "source,target,type\nA,B,rel\n";
        let doc = CsvParser.parse(csv, "d1").unwrap();
        assert_eq!(doc.relations[0].confidence, 1.0);
    }

    #[test]
    fn skips_empty_name_rows() {
        let csv = "name,type\n,node\nB,node\n";
        let doc = CsvParser.parse(csv, "d1").unwrap();
        assert_eq!(doc.entities.len(), 1);
        assert_eq!(doc.entities[0].name, "B");
    }
}
