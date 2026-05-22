use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;
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

impl fmt::Display for WikiCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_dir())
    }
}

impl FromStr for WikiCategory {
    type Err = WikiCategoryParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "summary" => Ok(Self::Summary),
            "entity" => Ok(Self::Entity),
            "topic" => Ok(Self::Topic),
            _ => Err(WikiCategoryParseError(s.to_string())),
        }
    }
}

#[derive(Debug, Clone, Error)]
#[error("unknown category: {0} (expected summary|entity|topic)")]
pub struct WikiCategoryParseError(pub String);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wiki_category_from_str_valid() {
        assert_eq!(
            "summary".parse::<WikiCategory>().unwrap(),
            WikiCategory::Summary
        );
        assert_eq!(
            "entity".parse::<WikiCategory>().unwrap(),
            WikiCategory::Entity
        );
        assert_eq!(
            "topic".parse::<WikiCategory>().unwrap(),
            WikiCategory::Topic
        );
    }

    #[test]
    fn from_str_rejects_invalid_category() {
        let err = WikiCategory::from_str("bogus").unwrap_err();
        assert!(err.to_string().contains("bogus"));
        assert!(err.to_string().contains("expected"));
    }

    #[test]
    fn wiki_category_display_roundtrip() {
        for cat in [
            WikiCategory::Summary,
            WikiCategory::Entity,
            WikiCategory::Topic,
        ] {
            let s = cat.to_string();
            let parsed: WikiCategory = s.parse().expect("roundtrip should work");
            assert_eq!(parsed, cat);
        }
    }

    #[test]
    fn wiki_category_as_dir() {
        assert_eq!(WikiCategory::Summary.as_dir(), "summary");
        assert_eq!(WikiCategory::Entity.as_dir(), "entity");
        assert_eq!(WikiCategory::Topic.as_dir(), "topic");
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
