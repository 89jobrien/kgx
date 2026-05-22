use std::path::{Path, PathBuf};

pub mod document;
pub mod graph;
pub mod ingest;
pub mod init;
pub mod types;
pub mod wiki;

pub use document::DocumentStore;
pub use graph::{EdgeInput, GraphStore};
pub use ingest::{IngestEntity, IngestRelation, ingest_entities, ingest_relations};
pub use init::init_workspace;
pub use types::{
    Chunk, ChunkId, DocId, Document, EdgeId, Entity, LintReport, MAX_GRAPH_DEPTH, MAX_NODES,
    MIN_CONFIDENCE, NodeId, QueryResult, Relation, WikiCategory, WikiCategoryParseError, WikiPage,
};
pub use wiki::WikiStore;

/// Standard path for the graph store within a workspace root.
pub fn graph_path(root: &Path) -> PathBuf {
    root.join("data").join("graph.json")
}

/// Standard path for the document store within a workspace root.
pub fn docs_path(root: &Path) -> PathBuf {
    root.join("data").join("documents.json")
}

/// Standard path for the wiki directory within a workspace root.
pub fn wiki_path(root: &Path) -> PathBuf {
    root.join("wiki")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn path_helpers() {
        let root = Path::new("/tmp/kgx");
        assert_eq!(graph_path(root), Path::new("/tmp/kgx/data/graph.json"));
        assert_eq!(docs_path(root), Path::new("/tmp/kgx/data/documents.json"));
        assert_eq!(wiki_path(root), Path::new("/tmp/kgx/wiki"));
    }
}
