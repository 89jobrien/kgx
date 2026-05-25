use super::GraphOp;
use crate::{EdgeInput, GraphStore};

/// Merge another graph into the target.
pub struct MergeOp {
    pub source: GraphStore,
}

/// Result of a merge operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeSummary {
    pub nodes_added: usize,
    pub nodes_merged: usize,
    pub edges_added: usize,
}

impl GraphOp for MergeOp {
    type Output = MergeSummary;

    fn apply(self, graph: &mut GraphStore) -> anyhow::Result<MergeSummary> {
        let mut nodes_added: usize = 0;
        let mut nodes_merged: usize = 0;
        let mut edges_added: usize = 0;

        let source_nodes: Vec<_> = self.source.nodes().cloned().collect();
        for node in &source_nodes {
            let existed = graph.node_by_name(&node.name).is_some();
            graph.add_node(
                &node.name,
                &node.entity_type,
                node.supporting_text.as_deref(),
                None,
            );
            // Merge source_docs from the source node
            for doc in &node.source_docs {
                graph.add_node(
                    &node.name,
                    &node.entity_type,
                    node.supporting_text.as_deref(),
                    Some(doc),
                );
            }
            if existed {
                nodes_merged += 1;
            } else {
                nodes_added += 1;
            }
        }

        // Build edge dedup set: (source_name, target_name, relation_type)
        let existing_edges: std::collections::HashSet<(String, String, String)> = graph
            .edges()
            .iter()
            .filter_map(|e| {
                let src = graph.get_node(e.source)?;
                let tgt = graph.get_node(e.target)?;
                Some((
                    src.name.to_lowercase(),
                    tgt.name.to_lowercase(),
                    e.relation_type.clone(),
                ))
            })
            .collect();

        for edge in self.source.edges() {
            let src_name = self
                .source
                .get_node(edge.source)
                .map(|n| n.name.clone())
                .unwrap_or_default();
            let tgt_name = self
                .source
                .get_node(edge.target)
                .map(|n| n.name.clone())
                .unwrap_or_default();
            let key = (
                src_name.to_lowercase(),
                tgt_name.to_lowercase(),
                edge.relation_type.clone(),
            );
            if existing_edges.contains(&key) {
                continue;
            }
            let Some(src_id) = graph.node_by_name(&src_name) else {
                continue;
            };
            let Some(tgt_id) = graph.node_by_name(&tgt_name) else {
                continue;
            };
            if graph
                .add_edge(EdgeInput {
                    source: src_id,
                    target: tgt_id,
                    relation_type: &edge.relation_type,
                    confidence: edge.confidence,
                    supporting_text: edge.supporting_text.as_deref(),
                    source_doc: edge.source_doc.as_deref(),
                })
                .is_some()
            {
                edges_added += 1;
            }
        }

        Ok(MergeSummary {
            nodes_added,
            nodes_merged,
            edges_added,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::conformance;
    use crate::{EdgeInput, GraphStore};

    fn fresh_graph() -> GraphStore {
        GraphStore::open(format!("/tmp/kgx_test_merge_{}.json", uuid::Uuid::new_v4())).unwrap()
    }

    #[test]
    fn merge_op_satisfies_contract() {
        conformance::assert_graph_op_contract(
            || MergeOp {
                source: fresh_graph(),
            },
            || {
                let mut s = fresh_graph();
                s.add_node("Gamma", "concept", None, None);
                MergeOp { source: s }
            },
            "merge",
        );
    }

    #[test]
    fn merge_adds_new_nodes() {
        let mut target = fresh_graph();
        target.add_node("A", "t", None, None);

        let mut source = fresh_graph();
        source.add_node("B", "t", None, None);

        let op = MergeOp { source };
        let summary = op.apply(&mut target).unwrap();
        assert_eq!(summary.nodes_added, 1);
        assert_eq!(summary.nodes_merged, 0);
        assert_eq!(target.node_count(), 2);
    }

    #[test]
    fn merge_deduplicates_by_name() {
        let mut target = fresh_graph();
        target.add_node("A", "t", None, Some("d1"));

        let mut source = fresh_graph();
        source.add_node("a", "t", None, Some("d2"));

        let op = MergeOp { source };
        let summary = op.apply(&mut target).unwrap();
        assert_eq!(summary.nodes_added, 0);
        assert_eq!(summary.nodes_merged, 1);
        assert_eq!(target.node_count(), 1);
        let id = target.node_by_name("a").unwrap();
        let node = target.get_node(id).unwrap();
        assert!(node.source_docs.contains(&"d1".to_string()));
        assert!(node.source_docs.contains(&"d2".to_string()));
    }

    #[test]
    fn merge_adds_edges() {
        let mut target = fresh_graph();
        let a = target.add_node("A", "t", None, None);
        let b = target.add_node("B", "t", None, None);
        target.add_edge(EdgeInput {
            source: a,
            target: b,
            relation_type: "r1",
            confidence: 1.0,
            supporting_text: None,
            source_doc: None,
        });

        let mut source = fresh_graph();
        let sa = source.add_node("A", "t", None, None);
        let sc = source.add_node("C", "t", None, None);
        source.add_edge(EdgeInput {
            source: sa,
            target: sc,
            relation_type: "r2",
            confidence: 0.9,
            supporting_text: None,
            source_doc: None,
        });

        let op = MergeOp { source };
        let summary = op.apply(&mut target).unwrap();
        assert_eq!(summary.edges_added, 1);
        assert_eq!(target.edge_count(), 2);
        assert_eq!(target.node_count(), 3);
    }

    #[test]
    fn merge_skips_duplicate_edges() {
        let mut target = fresh_graph();
        let a = target.add_node("A", "t", None, None);
        let b = target.add_node("B", "t", None, None);
        target.add_edge(EdgeInput {
            source: a,
            target: b,
            relation_type: "rel",
            confidence: 1.0,
            supporting_text: None,
            source_doc: None,
        });

        let mut source = fresh_graph();
        let sa = source.add_node("A", "t", None, None);
        let sb = source.add_node("B", "t", None, None);
        source.add_edge(EdgeInput {
            source: sa,
            target: sb,
            relation_type: "rel",
            confidence: 0.8,
            supporting_text: None,
            source_doc: None,
        });

        let op = MergeOp { source };
        let summary = op.apply(&mut target).unwrap();
        assert_eq!(summary.edges_added, 0);
        assert_eq!(target.edge_count(), 1);
    }

    #[test]
    fn merge_low_confidence_edge_skipped() {
        let mut target = fresh_graph();
        target.add_node("A", "t", None, None);

        let mut source = fresh_graph();
        let sa = source.add_node("A", "t", None, None);
        let sb = source.add_node("B", "t", None, None);
        source.add_edge(EdgeInput {
            source: sa,
            target: sb,
            relation_type: "weak",
            confidence: 0.3,
            supporting_text: None,
            source_doc: None,
        });

        let op = MergeOp { source };
        let summary = op.apply(&mut target).unwrap();
        assert_eq!(summary.edges_added, 0);
    }
}
