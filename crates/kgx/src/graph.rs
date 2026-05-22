use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::PathBuf;

use crate::types::*;
use uuid::Uuid;

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

impl GraphStore {
    /// Open or create a graph store at the given path.
    pub fn open(path: impl Into<PathBuf>) -> anyhow::Result<Self> {
        let path = path.into();
        let data = if path.exists() {
            let raw = fs::read_to_string(&path)?;
            serde_json::from_str::<GraphData>(&raw)?
        } else {
            GraphData::default()
        };

        let mut name_index = HashMap::new();
        let mut nodes = HashMap::new();
        for node in data.nodes {
            name_index.insert(node.name.to_lowercase(), node.id);
            nodes.insert(node.id, node);
        }

        Ok(Self {
            path,
            nodes,
            edges: data.edges,
            name_index,
        })
    }

    /// Persist current state to disk.
    pub fn save(&self) -> anyhow::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = GraphData {
            nodes: self.nodes.values().cloned().collect(),
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
            if let Some(doc) = source_doc {
                if let Some(node) = self.nodes.get_mut(&id) {
                    let doc_s = doc.to_string();
                    if !node.source_docs.contains(&doc_s) {
                        node.source_docs.push(doc_s);
                    }
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
    pub fn add_edge(
        &mut self,
        source: NodeId,
        target: NodeId,
        relation_type: &str,
        confidence: f64,
        supporting_text: Option<&str>,
        source_doc: Option<&str>,
    ) -> Option<EdgeId> {
        if confidence < MIN_CONFIDENCE {
            return None;
        }
        let id = Uuid::new_v4();
        self.edges.push(Relation {
            id,
            source,
            target,
            relation_type: relation_type.to_string(),
            confidence,
            supporting_text: supporting_text.map(String::from),
            source_doc: source_doc.map(String::from),
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
            for edge in &self.edges {
                if edge.confidence < MIN_CONFIDENCE {
                    continue;
                }
                let neighbor = if edge.source == current {
                    Some(edge.target)
                } else if edge.target == current {
                    Some(edge.source)
                } else {
                    None
                };
                if let Some(n) = neighbor {
                    result_edges.push(edge.clone());
                    if visited.insert(n) {
                        queue.push_back((n, depth + 1));
                    }
                }
            }
        }
        (result_nodes, result_edges)
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
                        .map_or(false, |t| t.to_lowercase().contains(&q))
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

    #[test]
    fn add_and_dedup_nodes() {
        let mut g = GraphStore::open("/tmp/kgx_test_graph.json").unwrap();
        let id1 = g.add_node("Rust", "language", None, None);
        let id2 = g.add_node("rust", "language", None, None);
        assert_eq!(id1, id2);
        assert_eq!(g.node_count(), 1);
    }

    #[test]
    fn low_confidence_edge_rejected() {
        let mut g = GraphStore::open("/tmp/kgx_test_graph2.json").unwrap();
        let a = g.add_node("A", "t", None, None);
        let b = g.add_node("B", "t", None, None);
        let result = g.add_edge(a, b, "maybe", 0.3, None, None);
        assert!(result.is_none());
        assert_eq!(g.edge_count(), 0);
    }

    #[test]
    fn bfs_respects_depth() {
        let mut g = GraphStore::open("/tmp/kgx_test_graph3.json").unwrap();
        let a = g.add_node("A", "t", None, None);
        let b = g.add_node("B", "t", None, None);
        let c = g.add_node("C", "t", None, None);
        let d = g.add_node("D", "t", None, None);
        g.add_edge(a, b, "r", 1.0, None, None);
        g.add_edge(b, c, "r", 1.0, None, None);
        g.add_edge(c, d, "r", 1.0, None, None);
        let (nodes, _) = g.bfs_subgraph(a);
        // depth=0: A, depth=1: B, depth=2: C. D is at depth 3 — excluded.
        assert_eq!(nodes.len(), 3);
    }
}
