use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Retrieval constraints.
pub const MAX_GRAPH_DEPTH: usize = 2;
pub const MIN_CONFIDENCE: f64 = 0.6;
pub const MAX_NODES: usize = 50;

/// A unique identifier for graph objects.
pub type NodeId = Uuid;
pub type EdgeId = Uuid;
pub type DocId = String;
pub type ChunkId = Uuid;

/// An entity node in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: NodeId,
    pub name: String,
    #[serde(rename = "type")]
    pub entity_type: String,
    pub supporting_text: Option<String>,
    /// Which document(s) this entity was extracted from.
    pub source_docs: Vec<DocId>,
}

/// A directed relation between two entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub id: EdgeId,
    pub source: NodeId,
    pub target: NodeId,
    #[serde(rename = "type")]
    pub relation_type: String,
    pub confidence: f64,
    pub supporting_text: Option<String>,
    pub source_doc: Option<DocId>,
}

/// A raw ingested document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: DocId,
    pub title: String,
    pub source: String,
    pub raw_content: String,
    pub chunks: Vec<Chunk>,
}

/// A chunk of a document, used for provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub id: ChunkId,
    pub doc_id: DocId,
    pub text: String,
    pub offset: usize,
}

/// A wiki page stored as markdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiPage {
    pub slug: String,
    pub category: WikiCategory,
    pub title: String,
    pub content: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WikiCategory {
    Summary,
    Entity,
    Topic,
}

impl WikiCategory {
    pub fn as_dir(&self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::Entity => "entity",
            Self::Topic => "topic",
        }
    }
}

/// Result of a graph query with provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub query: String,
    pub nodes: Vec<Entity>,
    pub edges: Vec<Relation>,
    pub supporting_chunks: Vec<Chunk>,
}

/// Wiki lint issues.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LintReport {
    pub orphan_pages: Vec<String>,
    pub missing_pages: Vec<String>,
    pub broken_wikilinks: Vec<(String, String)>,
    pub isolated_pages: Vec<String>,
}
