use crate::graph::{EdgeInput, GraphStore};

/// An entity to ingest into the graph.
pub struct IngestEntity<'a> {
    pub name: &'a str,
    pub entity_type: &'a str,
    pub supporting_text: Option<&'a str>,
}

/// A relation to ingest into the graph.
pub struct IngestRelation<'a> {
    pub source: &'a str,
    pub target: &'a str,
    pub relation_type: &'a str,
    pub confidence: f64,
    pub supporting_text: Option<&'a str>,
}

/// Add entities to the graph, returning the count added.
pub fn ingest_entities(
    graph: &mut GraphStore,
    doc_id: &str,
    entities: &[IngestEntity<'_>],
) -> usize {
    for e in entities {
        graph.add_node(e.name, e.entity_type, e.supporting_text, Some(doc_id));
    }
    entities.len()
}

/// Add relations to the graph, returning the count added.
/// Returns `None` for a relation if its source or target entity
/// is not found.
pub fn ingest_relations(
    graph: &mut GraphStore,
    doc_id: &str,
    relations: &[IngestRelation<'_>],
) -> Result<usize, IngestRelationError> {
    let mut count = 0;
    for r in relations {
        let src = graph
            .node_by_name(r.source)
            .ok_or_else(|| IngestRelationError::UnknownEntity(r.source.to_string()))?;
        let tgt = graph
            .node_by_name(r.target)
            .ok_or_else(|| IngestRelationError::UnknownEntity(r.target.to_string()))?;
        if graph
            .add_edge(EdgeInput {
                source: src,
                target: tgt,
                relation_type: r.relation_type,
                confidence: r.confidence,
                supporting_text: r.supporting_text,
                source_doc: Some(doc_id),
            })
            .is_some()
        {
            count += 1;
        }
    }
    Ok(count)
}

#[derive(Debug, thiserror::Error)]
pub enum IngestRelationError {
    #[error("unknown entity: {0}")]
    UnknownEntity(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_graph() -> GraphStore {
        let path = format!("/tmp/kgx_test_ingest_{}.json", uuid::Uuid::new_v4());
        GraphStore::open(path).expect("fresh graph")
    }

    #[test]
    fn ingest_entities_adds_nodes() {
        let mut g = fresh_graph();
        let entities = vec![
            IngestEntity {
                name: "Rust",
                entity_type: "lang",
                supporting_text: None,
            },
            IngestEntity {
                name: "Go",
                entity_type: "lang",
                supporting_text: Some("gc"),
            },
        ];
        let count = ingest_entities(&mut g, "d1", &entities);
        assert_eq!(count, 2);
        assert_eq!(g.node_count(), 2);
    }

    #[test]
    fn ingest_relations_links_entities() {
        let mut g = fresh_graph();
        let entities = vec![
            IngestEntity {
                name: "A",
                entity_type: "t",
                supporting_text: None,
            },
            IngestEntity {
                name: "B",
                entity_type: "t",
                supporting_text: None,
            },
        ];
        ingest_entities(&mut g, "d1", &entities);

        let relations = vec![IngestRelation {
            source: "A",
            target: "B",
            relation_type: "rel",
            confidence: 0.9,
            supporting_text: None,
        }];
        let count = ingest_relations(&mut g, "d1", &relations).expect("should succeed");
        assert_eq!(count, 1);
        assert_eq!(g.edge_count(), 1);
    }

    #[test]
    fn ingest_relations_unknown_entity_fails() {
        let mut g = fresh_graph();
        g.add_node("A", "t", None, None);
        let relations = vec![IngestRelation {
            source: "A",
            target: "missing",
            relation_type: "rel",
            confidence: 1.0,
            supporting_text: None,
        }];
        let err = ingest_relations(&mut g, "d1", &relations).unwrap_err();
        assert!(err.to_string().contains("missing"));
    }

    #[test]
    fn ingest_relations_low_confidence_skipped() {
        let mut g = fresh_graph();
        let entities = vec![
            IngestEntity {
                name: "A",
                entity_type: "t",
                supporting_text: None,
            },
            IngestEntity {
                name: "B",
                entity_type: "t",
                supporting_text: None,
            },
        ];
        ingest_entities(&mut g, "d1", &entities);

        let relations = vec![IngestRelation {
            source: "A",
            target: "B",
            relation_type: "rel",
            confidence: 0.1,
            supporting_text: None,
        }];
        let count = ingest_relations(&mut g, "d1", &relations).expect("should succeed");
        assert_eq!(count, 0);
    }
}
