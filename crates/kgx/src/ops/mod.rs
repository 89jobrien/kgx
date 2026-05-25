pub mod merge;

use crate::GraphStore;

/// A graph operation (command pattern).
pub trait GraphOp {
    type Output;
    fn apply(self, graph: &mut GraphStore) -> anyhow::Result<Self::Output>;
}

/// Conformance test suite for the GraphOp trait.
/// Any implementation must:
/// 1. Not reduce node_count (operations should be additive or neutral)
/// 2. Return Ok (not panic) on an empty graph
/// 3. Return Ok (not panic) on a populated graph
#[cfg(test)]
pub mod conformance {
    use super::*;
    use crate::EdgeInput;

    pub fn empty_graph() -> GraphStore {
        GraphStore::open(format!(
            "/tmp/kgx_test_conformance_{}.json",
            uuid::Uuid::new_v4()
        ))
        .expect("fresh graph should open")
    }

    pub fn populated_graph() -> GraphStore {
        let mut g = empty_graph();
        let a = g.add_node("Alpha", "concept", Some("first"), Some("d1"));
        let b = g.add_node("Beta", "concept", Some("second"), Some("d1"));
        g.add_edge(EdgeInput {
            source: a,
            target: b,
            relation_type: "precedes",
            confidence: 0.8,
            supporting_text: None,
            source_doc: Some("d1"),
        });
        g
    }

    /// Run the conformance suite. `make_op_empty` and `make_op_pop`
    /// are closures that produce a fresh op instance for each test
    /// scenario (since GraphOp consumes self).
    pub fn assert_graph_op_contract<T: GraphOp>(
        make_op_empty: impl FnOnce() -> T,
        make_op_pop: impl FnOnce() -> T,
        label: &str,
    ) {
        // 1. Succeeds on empty graph
        {
            let mut g = empty_graph();
            let op = make_op_empty();
            op.apply(&mut g)
                .unwrap_or_else(|e| panic!("{label}: must not error on empty graph: {e}"));
        }

        // 2. Succeeds on populated graph without reducing node count
        {
            let mut g = populated_graph();
            let before = g.node_count();
            let op = make_op_pop();
            op.apply(&mut g)
                .unwrap_or_else(|e| panic!("{label}: must not error on populated graph: {e}"));
            assert!(
                g.node_count() >= before,
                "{label}: node_count must not decrease (was {before}, now {})",
                g.node_count()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct CountNodes;

    impl GraphOp for CountNodes {
        type Output = usize;

        fn apply(self, graph: &mut GraphStore) -> anyhow::Result<usize> {
            Ok(graph.node_count())
        }
    }

    #[test]
    fn graph_op_trait_works() {
        let mut g =
            GraphStore::open(format!("/tmp/kgx_test_op_{}.json", uuid::Uuid::new_v4())).unwrap();
        g.add_node("A", "t", None, None);
        let count = CountNodes.apply(&mut g).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn count_nodes_satisfies_contract() {
        conformance::assert_graph_op_contract(|| CountNodes, || CountNodes, "count_nodes");
    }
}
