use super::GraphOp;
use crate::GraphStore;
/// Compare target graph against a baseline, producing a diff.
/// The baseline is consumed; the target graph is not modified.
pub struct DiffOp {
    pub baseline: GraphStore,
}

/// Result of diffing two graphs.
#[derive(Debug, Clone, Default)]
pub struct GraphDiff {
    pub nodes_added: Vec<String>,
    pub nodes_removed: Vec<String>,
    pub nodes_shared: Vec<String>,
    pub edges_added: Vec<EdgeDiff>,
    pub edges_removed: Vec<EdgeDiff>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EdgeDiff {
    pub source: String,
    pub target: String,
    pub relation_type: String,
}

impl GraphOp for DiffOp {
    type Output = GraphDiff;

    fn apply(self, graph: &mut GraphStore) -> anyhow::Result<GraphDiff> {
        let target_names: std::collections::HashSet<String> =
            graph.nodes().map(|n| n.name.to_lowercase()).collect();
        let baseline_names: std::collections::HashSet<String> = self
            .baseline
            .nodes()
            .map(|n| n.name.to_lowercase())
            .collect();

        let mut nodes_added: Vec<String> =
            target_names.difference(&baseline_names).cloned().collect();
        nodes_added.sort();

        let mut nodes_removed: Vec<String> =
            baseline_names.difference(&target_names).cloned().collect();
        nodes_removed.sort();

        let mut nodes_shared: Vec<String> = target_names
            .intersection(&baseline_names)
            .cloned()
            .collect();
        nodes_shared.sort();

        let target_edges = edge_set(graph);
        let baseline_edges = edge_set(&self.baseline);

        let mut edges_added: Vec<EdgeDiff> =
            target_edges.difference(&baseline_edges).cloned().collect();
        edges_added.sort_by(|a, b| (&a.source, &a.target).cmp(&(&b.source, &b.target)));

        let mut edges_removed: Vec<EdgeDiff> =
            baseline_edges.difference(&target_edges).cloned().collect();
        edges_removed.sort_by(|a, b| (&a.source, &a.target).cmp(&(&b.source, &b.target)));

        Ok(GraphDiff {
            nodes_added,
            nodes_removed,
            nodes_shared,
            edges_added,
            edges_removed,
        })
    }
}

fn edge_set(graph: &GraphStore) -> std::collections::HashSet<EdgeDiff> {
    graph
        .edges()
        .iter()
        .filter_map(|e| {
            let src = graph.get_node(e.source)?;
            let tgt = graph.get_node(e.target)?;
            Some(EdgeDiff {
                source: src.name.to_lowercase(),
                target: tgt.name.to_lowercase(),
                relation_type: e.relation_type.clone(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::conformance;
    use crate::{EdgeInput, GraphStore};

    fn fresh_graph() -> GraphStore {
        GraphStore::open(format!("/tmp/kgx_test_diff_{}.json", uuid::Uuid::new_v4())).unwrap()
    }

    #[test]
    fn diff_op_satisfies_contract() {
        conformance::assert_graph_op_contract(
            || DiffOp {
                baseline: fresh_graph(),
            },
            || DiffOp {
                baseline: fresh_graph(),
            },
            "diff",
        );
    }

    #[test]
    fn diff_identical_graphs() {
        let mut a = fresh_graph();
        a.add_node("A", "t", None, None);
        let mut b = fresh_graph();
        b.add_node("A", "t", None, None);

        let diff = DiffOp { baseline: b }.apply(&mut a).unwrap();
        assert!(diff.nodes_added.is_empty());
        assert!(diff.nodes_removed.is_empty());
        assert_eq!(diff.nodes_shared, vec!["a"]);
    }

    #[test]
    fn diff_added_and_removed_nodes() {
        let mut target = fresh_graph();
        target.add_node("A", "t", None, None);
        target.add_node("B", "t", None, None);

        let mut baseline = fresh_graph();
        baseline.add_node("A", "t", None, None);
        baseline.add_node("C", "t", None, None);

        let diff = DiffOp { baseline }.apply(&mut target).unwrap();
        assert_eq!(diff.nodes_added, vec!["b"]);
        assert_eq!(diff.nodes_removed, vec!["c"]);
        assert_eq!(diff.nodes_shared, vec!["a"]);
    }

    #[test]
    fn diff_edges() {
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

        let mut baseline = fresh_graph();
        let ba = baseline.add_node("A", "t", None, None);
        let bb = baseline.add_node("B", "t", None, None);
        baseline.add_edge(EdgeInput {
            source: ba,
            target: bb,
            relation_type: "r2",
            confidence: 1.0,
            supporting_text: None,
            source_doc: None,
        });

        let diff = DiffOp { baseline }.apply(&mut target).unwrap();
        assert_eq!(diff.edges_added.len(), 1);
        assert_eq!(diff.edges_added[0].relation_type, "r1");
        assert_eq!(diff.edges_removed.len(), 1);
        assert_eq!(diff.edges_removed[0].relation_type, "r2");
    }

    #[test]
    fn diff_empty_graphs() {
        let target = fresh_graph();
        let baseline = fresh_graph();
        let diff = DiffOp { baseline }.apply(&mut { target }).unwrap();
        assert!(diff.nodes_added.is_empty());
        assert!(diff.nodes_removed.is_empty());
        assert!(diff.edges_added.is_empty());
        assert!(diff.edges_removed.is_empty());
    }
}
