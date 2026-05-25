use super::{ParseError, ParsedDocument, ParsedEntity, ParsedRelation, Parser};

/// Parses markdown into entities (headings) and relations (links).
pub struct MarkdownParser;

impl Parser for MarkdownParser {
    fn parse(&self, input: &str, doc_id: &str) -> Result<ParsedDocument, ParseError> {
        let mut entities = Vec::new();
        let mut relations = Vec::new();
        let mut current_heading: Option<String> = None;

        for line in input.lines() {
            if let Some(title) = line.strip_prefix("## ") {
                let title = title.trim().to_string();
                entities.push(ParsedEntity {
                    name: title.clone(),
                    entity_type: "heading".to_string(),
                    supporting_text: None,
                });
                current_heading = Some(title);
            } else if let Some(title) = line.strip_prefix("# ") {
                let title = title.trim().to_string();
                entities.push(ParsedEntity {
                    name: title.clone(),
                    entity_type: "section".to_string(),
                    supporting_text: None,
                });
                current_heading = Some(title);
            } else if let Some(ref heading) = current_heading {
                extract_wikilinks(line, heading, &mut relations);
                extract_md_links(line, heading, &mut relations);
            }
        }

        Ok(ParsedDocument {
            doc_id: doc_id.to_string(),
            entities,
            relations,
        })
    }
}

fn extract_wikilinks(line: &str, heading: &str, relations: &mut Vec<ParsedRelation>) {
    let mut rest = line;
    while let Some(start) = rest.find("[[") {
        let after = &rest[start + 2..];
        if let Some(end) = after.find("]]") {
            let target = &after[..end];
            if !target.is_empty() {
                relations.push(ParsedRelation {
                    source: heading.to_string(),
                    target: target.to_string(),
                    relation_type: "references".to_string(),
                    confidence: 1.0,
                    supporting_text: None,
                });
            }
            rest = &after[end + 2..];
        } else {
            break;
        }
    }
}

fn extract_md_links(line: &str, heading: &str, relations: &mut Vec<ParsedRelation>) {
    let mut rest = line;
    while let Some(bracket_start) = rest.find('[') {
        let after_bracket = &rest[bracket_start + 1..];
        if let Some(bracket_end) = after_bracket.find("](") {
            let paren_content = &after_bracket[bracket_end + 2..];
            if let Some(paren_end) = paren_content.find(')') {
                let url = &paren_content[..paren_end];
                if !url.starts_with("http://") && !url.starts_with("https://") && !url.is_empty() {
                    relations.push(ParsedRelation {
                        source: heading.to_string(),
                        target: url.to_string(),
                        relation_type: "links_to".to_string(),
                        confidence: 0.8,
                        supporting_text: None,
                    });
                }
                rest = &paren_content[paren_end + 1..];
            } else {
                break;
            }
        } else {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::conformance;

    #[test]
    fn markdown_parser_satisfies_contract() {
        conformance::assert_parser_contract(&MarkdownParser, "markdown");
    }

    #[test]
    fn headings_become_entities() {
        let md = "# Top\n\nSome text.\n\n## Sub\n\nMore text.\n";
        let p = MarkdownParser;
        let doc = p.parse(md, "d1").unwrap();
        let names: Vec<&str> = doc.entities.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"Top"));
        assert!(names.contains(&"Sub"));
        let top = doc.entities.iter().find(|e| e.name == "Top").unwrap();
        assert_eq!(top.entity_type, "section");
        let sub = doc.entities.iter().find(|e| e.name == "Sub").unwrap();
        assert_eq!(sub.entity_type, "heading");
    }

    #[test]
    fn wikilinks_become_relations() {
        let md = "## Source\n\nSee [[Target]] for details.\n";
        let p = MarkdownParser;
        let doc = p.parse(md, "d1").unwrap();
        assert_eq!(doc.relations.len(), 1);
        let r = &doc.relations[0];
        assert_eq!(r.source, "Source");
        assert_eq!(r.target, "Target");
        assert_eq!(r.relation_type, "references");
        assert_eq!(r.confidence, 1.0);
    }

    #[test]
    fn internal_links_become_relations() {
        let md = "## Page\n\nSee [other](other-page.md) here.\n";
        let p = MarkdownParser;
        let doc = p.parse(md, "d1").unwrap();
        assert_eq!(doc.relations.len(), 1);
        let r = &doc.relations[0];
        assert_eq!(r.source, "Page");
        assert_eq!(r.target, "other-page.md");
        assert_eq!(r.relation_type, "links_to");
        assert_eq!(r.confidence, 0.8);
    }

    #[test]
    fn external_links_ignored() {
        let md = "## Page\n\nSee [rust](https://rust-lang.org) here.\n";
        let p = MarkdownParser;
        let doc = p.parse(md, "d1").unwrap();
        assert!(doc.relations.is_empty());
    }

    #[test]
    fn no_headings_yields_empty() {
        let md = "Just some text without headings.\n";
        let p = MarkdownParser;
        let doc = p.parse(md, "d1").unwrap();
        assert!(doc.entities.is_empty());
        assert!(doc.relations.is_empty());
    }

    #[test]
    fn multiple_wikilinks_in_one_section() {
        let md = "## Hub\n\n[[A]] and [[B]] are linked.\n";
        let p = MarkdownParser;
        let doc = p.parse(md, "d1").unwrap();
        assert_eq!(doc.relations.len(), 2);
        assert!(doc.relations.iter().all(|r| r.source == "Hub"));
        let targets: Vec<&str> = doc.relations.iter().map(|r| r.target.as_str()).collect();
        assert!(targets.contains(&"A"));
        assert!(targets.contains(&"B"));
    }

    mod prop {
        use super::*;
        use proptest::prelude::*;

        fn arb_markdown() -> impl Strategy<Value = String> {
            proptest::collection::vec(
                (
                    "[A-Za-z][A-Za-z0-9 ]{0,20}",
                    proptest::collection::vec("[A-Za-z][A-Za-z0-9]{0,10}", 0..=3),
                ),
                1..=5,
            )
            .prop_map(|sections| {
                let mut md = String::new();
                for (heading, links) in sections {
                    md.push_str(&format!("## {heading}\n\n"));
                    for link in links {
                        md.push_str(&format!("See [[{link}]].\n"));
                    }
                    md.push('\n');
                }
                md
            })
        }

        proptest! {
            #[test]
            fn markdown_prop_never_panics(input in ".*") {
                let p = MarkdownParser;
                let _ = p.parse(&input, "prop-doc");
            }

            #[test]
            fn markdown_prop_relation_sources_are_entities(
                md in arb_markdown()
            ) {
                let p = MarkdownParser;
                let doc = p.parse(&md, "prop-doc")
                    .expect("structured markdown must parse");
                let entity_names: std::collections::HashSet<&str> = doc
                    .entities
                    .iter()
                    .map(|e| e.name.as_str())
                    .collect();
                for rel in &doc.relations {
                    prop_assert!(
                        entity_names.contains(rel.source.as_str()),
                        "relation source '{}' not in entities",
                        rel.source
                    );
                }
            }

            #[test]
            fn markdown_prop_confidence_in_range(
                md in arb_markdown()
            ) {
                let p = MarkdownParser;
                let doc = p.parse(&md, "prop-doc")
                    .expect("structured markdown must parse");
                for rel in &doc.relations {
                    prop_assert!(
                        (0.0..=1.0).contains(&rel.confidence),
                        "confidence {} out of range",
                        rel.confidence
                    );
                }
            }

            #[test]
            fn markdown_prop_doc_id_preserved(
                md in arb_markdown(),
                doc_id in "[a-z]{1,10}"
            ) {
                let p = MarkdownParser;
                let doc = p.parse(&md, &doc_id)
                    .expect("structured markdown must parse");
                prop_assert_eq!(doc.doc_id, doc_id);
            }
        }
    }
}
