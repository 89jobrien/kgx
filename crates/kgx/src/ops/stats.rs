use super::GraphOp;
use crate::GraphStore;
use crate::types::NodeId;
use std::collections::{HashMap, HashSet};

/// Compute graph analytics: degree centrality, connected components,
/// and orphan detection.
pub struct StatsOp;

/// Graph statistics.
#[derive(Debug, Clone)]
pub struct GraphStats {
    pub node_count: usize,
    pub edge_count: usize,
    /// Degree centrality per node: (name, in_degree, out_degree, total_degree).
    pub degree_centrality: Vec<DegreeCentrality>,
    /// Connected components as lists of node names.
    pub components: Vec<Vec<String>>,
    /// Nodes with zero edges.
    pub orphans: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DegreeCentrality {
    pub name: String,
    pub in_degree: usize,
    pub out_degree: usize,
    pub total_degree: usize,
}

impl GraphOp for StatsOp {
    type Output = GraphStats;

    fn apply(self, graph: &mut GraphStore) -> anyhow::Result<GraphStats> {
        let node_count = graph.node_count();
        let edge_count = graph.edge_count();

        // Degree centrality
        let mut in_deg: HashMap<NodeId, usize> = HashMap::new();
        let mut out_deg: HashMap<NodeId, usize> = HashMap::new();

        for edge in graph.edges() {
            *out_deg.entry(edge.source).or_default() += 1;
            *in_deg.entry(edge.target).or_default() += 1;
        }

        let mut degree_centrality: Vec<DegreeCentrality> = graph
            .nodes()
            .map(|n| {
                let i = in_deg.get(&n.id).copied().unwrap_or(0);
                let o = out_deg.get(&n.id).copied().unwrap_or(0);
                DegreeCentrality {
                    name: n.name.clone(),
                    in_degree: i,
                    out_degree: o,
                    total_degree: i + o,
                }
            })
            .collect();
        degree_centrality.sort_by(|a, b| {
            b.total_degree
                .cmp(&a.total_degree)
                .then(a.name.cmp(&b.name))
        });

        // Orphans
        let mut orphans: Vec<String> = degree_centrality
            .iter()
            .filter(|d| d.total_degree == 0)
            .map(|d| d.name.clone())
            .collect();
        orphans.sort();

        // Connected components (undirected view)
        let all_ids: Vec<NodeId> = graph.nodes().map(|n| n.id).collect();
        let mut visited: HashSet<NodeId> = HashSet::new();
        let mut components: Vec<Vec<String>> = Vec::new();

        // Build adjacency list (undirected)
        let mut adj: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        for edge in graph.edges() {
            adj.entry(edge.source).or_default().push(edge.target);
            adj.entry(edge.target).or_default().push(edge.source);
        }

        for id in &all_ids {
            if visited.contains(id) {
                continue;
            }
            let mut component = Vec::new();
            let mut stack = vec![*id];
            while let Some(current) = stack.pop() {
                if !visited.insert(current) {
                    continue;
                }
                if let Some(node) = graph.get_node(current) {
                    component.push(node.name.clone());
                }
                if let Some(neighbors) = adj.get(&current) {
                    for &neighbor in neighbors {
                        if !visited.contains(&neighbor) {
                            stack.push(neighbor);
                        }
                    }
                }
            }
            component.sort();
            components.push(component);
        }
        components.sort_by(|a, b| b.len().cmp(&a.len()).then(a.cmp(b)));

        Ok(GraphStats {
            node_count,
            edge_count,
            degree_centrality,
            components,
            orphans,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::conformance;
    use crate::{EdgeInput, GraphStore};

    fn fresh_graph() -> GraphStore {
        GraphStore::open(format!("/tmp/kgx_test_stats_{}.json", uuid::Uuid::new_v4())).unwrap()
    }

    #[test]
    fn stats_op_satisfies_contract() {
        conformance::assert_graph_op_contract(|| StatsOp, || StatsOp, "stats");
    }

    #[test]
    fn empty_graph_stats() {
        let mut g = fresh_graph();
        let stats = StatsOp.apply(&mut g).unwrap();
        assert_eq!(stats.node_count, 0);
        assert_eq!(stats.edge_count, 0);
        assert!(stats.degree_centrality.is_empty());
        assert!(stats.components.is_empty());
        assert!(stats.orphans.is_empty());
    }

    #[test]
    fn orphan_detection() {
        let mut g = fresh_graph();
        let a = g.add_node("A", "t", None, None);
        let b = g.add_node("B", "t", None, None);
        g.add_node("C", "t", None, None); // orphan
        g.add_edge(EdgeInput {
            source: a,
            target: b,
            relation_type: "r",
            confidence: 1.0,
            supporting_text: None,
            source_doc: None,
        });

        let stats = StatsOp.apply(&mut g).unwrap();
        assert_eq!(stats.orphans, vec!["C"]);
    }

    #[test]
    fn degree_centrality() {
        let mut g = fresh_graph();
        let a = g.add_node("A", "t", None, None);
        let b = g.add_node("B", "t", None, None);
        let c = g.add_node("C", "t", None, None);
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

        let stats = StatsOp.apply(&mut g).unwrap();
        let a_deg = stats
            .degree_centrality
            .iter()
            .find(|d| d.name == "A")
            .unwrap();
        assert_eq!(a_deg.out_degree, 2);
        assert_eq!(a_deg.in_degree, 0);
        assert_eq!(a_deg.total_degree, 2);

        let b_deg = stats
            .degree_centrality
            .iter()
            .find(|d| d.name == "B")
            .unwrap();
        assert_eq!(b_deg.in_degree, 1);
        assert_eq!(b_deg.out_degree, 0);
    }

    #[test]
    fn connected_components() {
        let mut g = fresh_graph();
        let a = g.add_node("A", "t", None, None);
        let b = g.add_node("B", "t", None, None);
        let c = g.add_node("C", "t", None, None);
        let d = g.add_node("D", "t", None, None);
        g.add_edge(EdgeInput {
            source: a,
            target: b,
            relation_type: "r",
            confidence: 1.0,
            supporting_text: None,
            source_doc: None,
        });
        g.add_edge(EdgeInput {
            source: c,
            target: d,
            relation_type: "r",
            confidence: 1.0,
            supporting_text: None,
            source_doc: None,
        });

        let stats = StatsOp.apply(&mut g).unwrap();
        assert_eq!(stats.components.len(), 2);
        assert_eq!(stats.components[0].len(), 2);
        assert_eq!(stats.components[1].len(), 2);
    }

    #[test]
    fn degree_sorted_descending() {
        let mut g = fresh_graph();
        let a = g.add_node("A", "t", None, None);
        let b = g.add_node("B", "t", None, None);
        let c = g.add_node("C", "t", None, None);
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
        g.add_edge(EdgeInput {
            source: b,
            target: c,
            relation_type: "r",
            confidence: 1.0,
            supporting_text: None,
            source_doc: None,
        });

        let stats = StatsOp.apply(&mut g).unwrap();
        let degrees: Vec<usize> = stats
            .degree_centrality
            .iter()
            .map(|d| d.total_degree)
            .collect();
        assert!(degrees.windows(2).all(|w| w[0] >= w[1]));
    }
}
