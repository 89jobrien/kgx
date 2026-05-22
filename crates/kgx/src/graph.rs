use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::PathBuf;

use crate::types::{EdgeId, Entity, MAX_GRAPH_DEPTH, MAX_NODES, MIN_CONFIDENCE, NodeId, Relation};
use uuid::Uuid;

/// Input for adding an edge, reducing parameter count on `add_edge`.
pub struct EdgeInput<'a> {
    pub source: NodeId,
    pub target: NodeId,
    pub relation_type: &'a str,
    pub confidence: f64,
    pub supporting_text: Option<&'a str>,
    pub source_doc: Option<&'a str>,
}

/// Persistent entity-relation graph backed by a JSON file.
#[derive(Debug)]
pub struct GraphStore {
    path: PathBuf,
    nodes: HashMap<NodeId, Entity>,
    edges: Vec<Relation>,
    /// name (lowercased) -> NodeId for dedup.
    name_index: HashMap<String, NodeId>,
}

/// On-disk format.
#[derive(serde::Serialize, serde::Deserialize, Default)]
struct GraphData {
    nodes: Vec<Entity>,
    edges: Vec<Relation>,
}

// qual:allow(iosp) reason: "I/O boundary — must check existence then read"
fn read_graph_data(path: &std::path::Path) -> anyhow::Result<GraphData> {
    if path.exists() {
        let raw = fs::read_to_string(path)?;
        Ok(serde_json::from_str::<GraphData>(&raw)?)
    } else {
        Ok(GraphData::default())
    }
}

impl GraphStore {
    /// Open or create a graph store at the given path.
    pub fn open(path: impl Into<PathBuf>) -> anyhow::Result<Self> {
        let path = path.into();
        let data = read_graph_data(&path)?;
        Ok(Self::from_data(path, data))
    }

    fn from_data(path: PathBuf, data: GraphData) -> Self {
        let mut name_index = HashMap::new();
        let mut nodes = HashMap::new();
        for node in data.nodes {
            name_index.insert(node.name.to_lowercase(), node.id);
            nodes.insert(node.id, node);
        }
        Self {
            path,
            nodes,
            edges: data.edges,
            name_index,
        }
    }

    /// Persist current state to disk.
    pub fn save(&self) -> anyhow::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut nodes: Vec<_> = self.nodes.values().cloned().collect();
        nodes.sort_by(|a, b| a.name.cmp(&b.name));
        let data = GraphData {
            nodes,
            edges: self.edges.clone(),
        };
        let json = serde_json::to_string_pretty(&data)?;
        fs::write(&self.path, json)?;
        Ok(())
    }

    /// Add an entity node. Deduplicates by lowercased name.
    /// Returns the node ID (existing or new).
    pub fn add_node(
        &mut self,
        name: &str,
        entity_type: &str,
        supporting_text: Option<&str>,
        source_doc: Option<&str>,
    ) -> NodeId {
        let key = name.to_lowercase();
        if let Some(&id) = self.name_index.get(&key) {
            // Merge source doc if new.
            if let Some(doc) = source_doc
                && let Some(node) = self.nodes.get_mut(&id)
            {
                let doc_s = doc.to_string();
                if !node.source_docs.contains(&doc_s) {
                    node.source_docs.push(doc_s);
                }
            }
            return id;
        }

        let id = Uuid::new_v4();
        let entity = Entity {
            id,
            name: name.to_string(),
            entity_type: entity_type.to_string(),
            supporting_text: supporting_text.map(String::from),
            source_docs: source_doc.into_iter().map(String::from).collect(),
        };
        self.name_index.insert(key, id);
        self.nodes.insert(id, entity);
        id
    }

    /// Add a directed relation. Skips if confidence < MIN_CONFIDENCE.
    pub fn add_edge(&mut self, input: EdgeInput<'_>) -> Option<EdgeId> {
        if input.confidence < MIN_CONFIDENCE {
            return None;
        }
        let id = Uuid::new_v4();
        self.edges.push(Relation {
            id,
            source: input.source,
            target: input.target,
            relation_type: input.relation_type.to_string(),
            confidence: input.confidence,
            supporting_text: input.supporting_text.map(String::from),
            source_doc: input.source_doc.map(String::from),
        });
        Some(id)
    }

    /// Look up a node ID by name.
    pub fn node_by_name(&self, name: &str) -> Option<NodeId> {
        self.name_index.get(&name.to_lowercase()).copied()
    }

    /// Get a node by ID.
    pub fn get_node(&self, id: NodeId) -> Option<&Entity> {
        self.nodes.get(&id)
    }

    /// All nodes.
    pub fn nodes(&self) -> impl Iterator<Item = &Entity> {
        self.nodes.values()
    }

    /// All edges.
    pub fn edges(&self) -> &[Relation] {
        &self.edges
    }

    /// BFS subgraph extraction from a seed node, respecting
    /// MAX_GRAPH_DEPTH, MIN_CONFIDENCE, and MAX_NODES.
    pub fn bfs_subgraph(&self, seed: NodeId) -> (Vec<Entity>, Vec<Relation>) {
        let mut visited: HashSet<NodeId> = HashSet::new();
        let mut seen_edges: HashSet<EdgeId> = HashSet::new();
        let mut queue: VecDeque<(NodeId, usize)> = VecDeque::new();
        let mut result_nodes = Vec::new();
        let mut result_edges = Vec::new();

        queue.push_back((seed, 0));
        visited.insert(seed);

        while let Some((current, depth)) = queue.pop_front() {
            if result_nodes.len() >= MAX_NODES {
                break;
            }
            if let Some(node) = self.nodes.get(&current) {
                result_nodes.push(node.clone());
            }
            if depth >= MAX_GRAPH_DEPTH {
                continue;
            }
            for (edge, neighbor) in self.neighbors(current) {
                if seen_edges.insert(edge.id) {
                    result_edges.push(edge.clone());
                }
                if visited.insert(neighbor) {
                    queue.push_back((neighbor, depth + 1));
                }
            }
        }
        (result_nodes, result_edges)
    }

    /// Yield all edges incident to `node` that meet the confidence threshold,
    /// along with the neighbor node ID on the other end.
    fn neighbors(&self, node: NodeId) -> impl Iterator<Item = (&Relation, NodeId)> {
        self.edges.iter().filter_map(move |edge| {
            if edge.confidence < MIN_CONFIDENCE {
                return None;
            }
            if edge.source == node {
                Some((edge, edge.target))
            } else if edge.target == node {
                Some((edge, edge.source))
            } else {
                None
            }
        })
    }

    /// Search nodes by keyword in name or supporting_text.
    pub fn search(&self, query: &str) -> Vec<&Entity> {
        let q = query.to_lowercase();
        self.nodes
            .values()
            .filter(|n| {
                n.name.to_lowercase().contains(&q)
                    || n.supporting_text
                        .as_deref()
                        .is_some_and(|t| t.to_lowercase().contains(&q))
            })
            .take(MAX_NODES)
            .collect()
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create an in-memory graph (non-existent path, never saved).
    fn fresh_graph() -> GraphStore {
        let path = format!("/tmp/kgx_test_graph_{}.json", Uuid::new_v4());
        GraphStore::open(path).expect("fresh graph should open")
    }

    #[test]
    fn add_and_dedup_nodes() {
        let mut g = fresh_graph();
        let id1 = g.add_node("Rust", "language", None, None);
        let id2 = g.add_node("rust", "language", None, None);
        assert_eq!(id1, id2);
        assert_eq!(g.node_count(), 1);
    }

    #[test]
    fn add_node_merges_source_docs() {
        let mut g = fresh_graph();
        g.add_node("Rust", "language", None, Some("doc-1"));
        g.add_node("rust", "language", None, Some("doc-2"));
        // Same doc again -- should not duplicate.
        g.add_node("RUST", "language", None, Some("doc-1"));

        let id = g.node_by_name("rust").expect("node should exist");
        let node = g.get_node(id).expect("node should exist");
        assert_eq!(node.source_docs, vec!["doc-1", "doc-2"]);
    }

    fn edge(source: NodeId, target: NodeId, rel: &str, confidence: f64) -> EdgeInput<'_> {
        EdgeInput {
            source,
            target,
            relation_type: rel,
            confidence,
            supporting_text: None,
            source_doc: None,
        }
    }

    #[test]
    fn low_confidence_edge_rejected() {
        let mut g = fresh_graph();
        let a = g.add_node("A", "t", None, None);
        let b = g.add_node("B", "t", None, None);
        let result = g.add_edge(edge(a, b, "maybe", 0.3));
        assert!(result.is_none());
        assert_eq!(g.edge_count(), 0);
    }

    #[test]
    fn edge_at_exact_threshold_accepted() {
        let mut g = fresh_graph();
        let a = g.add_node("A", "t", None, None);
        let b = g.add_node("B", "t", None, None);
        let result = g.add_edge(edge(a, b, "rel", MIN_CONFIDENCE));
        assert!(result.is_some());
        assert_eq!(g.edge_count(), 1);
    }

    #[test]
    fn bfs_respects_depth() {
        let mut g = fresh_graph();
        let a = g.add_node("A", "t", None, None);
        let b = g.add_node("B", "t", None, None);
        let c = g.add_node("C", "t", None, None);
        let d = g.add_node("D", "t", None, None);
        g.add_edge(edge(a, b, "r", 1.0));
        g.add_edge(edge(b, c, "r", 1.0));
        g.add_edge(edge(c, d, "r", 1.0));
        let (nodes, _) = g.bfs_subgraph(a);
        // depth=0: A, depth=1: B, depth=2: C. D is at depth 3 -- excluded.
        assert_eq!(nodes.len(), 3);
    }

    /// Regression: BFS used to add duplicate edges when traversing from both
    /// endpoints. An A-B edge should appear exactly once.
    #[test]
    fn bfs_no_duplicate_edges() {
        let mut g = fresh_graph();
        let a = g.add_node("A", "t", None, None);
        let b = g.add_node("B", "t", None, None);
        g.add_edge(edge(a, b, "r", 1.0));
        let (nodes, edges) = g.bfs_subgraph(a);
        assert_eq!(nodes.len(), 2);
        assert_eq!(edges.len(), 1, "edge should not be duplicated");
    }

    #[test]
    fn save_and_reload_roundtrip() {
        let path = format!("/tmp/kgx_test_graph_rt_{}.json", Uuid::new_v4());
        {
            let mut g = GraphStore::open(&path).expect("open should work");
            let a = g.add_node("Alpha", "t", Some("first"), Some("d1"));
            let b = g.add_node("Beta", "t", None, None);
            g.add_edge(edge(a, b, "rel", 0.9));
            g.save().expect("save should work");
        }
        let g2 = GraphStore::open(&path).expect("reopen should work");
        assert_eq!(g2.node_count(), 2);
        assert_eq!(g2.edge_count(), 1);
        assert!(g2.node_by_name("alpha").is_some());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn edges_accessor() {
        let mut g = fresh_graph();
        let a = g.add_node("A", "t", None, None);
        let b = g.add_node("B", "t", None, None);
        g.add_edge(edge(a, b, "r", 1.0));
        assert_eq!(g.edges().len(), 1);
        assert_eq!(g.edges()[0].relation_type, "r");
    }

    #[test]
    fn search_matches_name_and_supporting_text() {
        let mut g = fresh_graph();
        g.add_node("Rust", "language", Some("systems programming"), None);
        g.add_node("Python", "language", Some("scripting language"), None);
        g.add_node("Cargo", "tool", Some("Rust package manager"), None);

        // "rust" matches "Rust" by name and "Cargo" by supporting_text.
        let by_rust = g.search("rust");
        let names: HashSet<&str> = by_rust.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(names.len(), 2);
        assert!(names.contains("Rust"));
        assert!(names.contains("Cargo"));

        let by_text = g.search("scripting");
        assert_eq!(by_text.len(), 1);
        assert_eq!(by_text[0].name, "Python");
    }
}
