use super::GraphOp;
use crate::GraphStore;
use crate::types::{Entity, NodeId, Relation};

/// Extract a subgraph by predicate. Matching nodes and their
/// interconnecting edges are returned. The target graph is not modified.
pub struct SubgraphOp {
    pub predicate: SubgraphPredicate,
}

/// Criteria for subgraph extraction.
pub enum SubgraphPredicate {
    /// Match entities by type (case-insensitive).
    EntityType(String),
    /// Match entities whose name contains the query (case-insensitive).
    NameContains(String),
    /// Match entities by a custom predicate.
    Custom(Box<dyn Fn(&Entity) -> bool + Send>),
}

/// Extracted subgraph.
#[derive(Debug, Clone)]
pub struct Subgraph {
    pub nodes: Vec<Entity>,
    pub edges: Vec<Relation>,
}

impl GraphOp for SubgraphOp {
    type Output = Subgraph;

    fn apply(self, graph: &mut GraphStore) -> anyhow::Result<Subgraph> {
        let matching_ids: std::collections::HashSet<NodeId> = graph
            .nodes()
            .filter(|n| match &self.predicate {
                SubgraphPredicate::EntityType(t) => n.entity_type.eq_ignore_ascii_case(t),
                SubgraphPredicate::NameContains(q) => {
                    n.name.to_lowercase().contains(&q.to_lowercase())
                }
                SubgraphPredicate::Custom(f) => f(n),
            })
            .map(|n| n.id)
            .collect();

        let nodes: Vec<Entity> = graph
            .nodes()
            .filter(|n| matching_ids.contains(&n.id))
            .cloned()
            .collect();

        let edges: Vec<Relation> = graph
            .edges()
            .iter()
            .filter(|e| matching_ids.contains(&e.source) && matching_ids.contains(&e.target))
            .cloned()
            .collect();

        Ok(Subgraph { nodes, edges })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::conformance;
    use crate::{EdgeInput, GraphStore};

    fn fresh_graph() -> GraphStore {
        GraphStore::open(format!(
            "/tmp/kgx_test_subgraph_{}.json",
            uuid::Uuid::new_v4()
        ))
        .unwrap()
    }

    #[test]
    fn subgraph_op_satisfies_contract() {
        conformance::assert_graph_op_contract(
            || SubgraphOp {
                predicate: SubgraphPredicate::EntityType("concept".to_string()),
            },
            || SubgraphOp {
                predicate: SubgraphPredicate::EntityType("concept".to_string()),
            },
            "subgraph",
        );
    }

    #[test]
    fn filter_by_entity_type() {
        let mut g = fresh_graph();
        g.add_node("Rust", "language", None, None);
        g.add_node("Cargo", "tool", None, None);
        g.add_node("Go", "language", None, None);

        let op = SubgraphOp {
            predicate: SubgraphPredicate::EntityType("language".to_string()),
        };
        let sub = op.apply(&mut g).unwrap();
        assert_eq!(sub.nodes.len(), 2);
        assert!(sub.nodes.iter().all(|n| n.entity_type == "language"));
    }

    #[test]
    fn filter_by_name_contains() {
        let mut g = fresh_graph();
        g.add_node("Rust", "lang", None, None);
        g.add_node("RustFmt", "tool", None, None);
        g.add_node("Go", "lang", None, None);

        let op = SubgraphOp {
            predicate: SubgraphPredicate::NameContains("rust".to_string()),
        };
        let sub = op.apply(&mut g).unwrap();
        assert_eq!(sub.nodes.len(), 2);
    }

    #[test]
    fn includes_interconnecting_edges_only() {
        let mut g = fresh_graph();
        let a = g.add_node("A", "t", None, None);
        let b = g.add_node("B", "t", None, None);
        let c = g.add_node("C", "other", None, None);
        g.add_edge(EdgeInput {
            source: a,
            target: b,
            relation_type: "r",
            confidence: 1.0,
            supporting_text: None,
            source_doc: None,
        });
        g.add_edge(EdgeInput {
            source: a,
            target: c,
            relation_type: "r",
            confidence: 1.0,
            supporting_text: None,
            source_doc: None,
        });

        let op = SubgraphOp {
            predicate: SubgraphPredicate::EntityType("t".to_string()),
        };
        let sub = op.apply(&mut g).unwrap();
        assert_eq!(sub.nodes.len(), 2);
        assert_eq!(sub.edges.len(), 1, "edge to C should be excluded");
    }

    #[test]
    fn custom_predicate() {
        let mut g = fresh_graph();
        g.add_node("Short", "t", None, None);
        g.add_node("VeryLongName", "t", None, None);

        let op = SubgraphOp {
            predicate: SubgraphPredicate::Custom(Box::new(|e| e.name.len() > 5)),
        };
        let sub = op.apply(&mut g).unwrap();
        assert_eq!(sub.nodes.len(), 1);
        assert_eq!(sub.nodes[0].name, "VeryLongName");
    }

    #[test]
    fn no_matches_returns_empty() {
        let mut g = fresh_graph();
        g.add_node("A", "t", None, None);

        let op = SubgraphOp {
            predicate: SubgraphPredicate::EntityType("nonexistent".to_string()),
        };
        let sub = op.apply(&mut g).unwrap();
        assert!(sub.nodes.is_empty());
        assert!(sub.edges.is_empty());
    }
}
