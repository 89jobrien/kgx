use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::types::*;
use uuid::Uuid;

/// Persistent raw document store backed by a JSON file.
#[derive(Debug)]
pub struct DocumentStore {
    path: PathBuf,
    docs: HashMap<DocId, Document>,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct DocsData {
    documents: Vec<Document>,
}

/// Default chunk size in characters.
const CHUNK_SIZE: usize = 1000;
/// Overlap between adjacent chunks.
const CHUNK_OVERLAP: usize = 200;

impl DocumentStore {
    pub fn open(path: impl Into<PathBuf>) -> anyhow::Result<Self> {
        let path = path.into();
        let data = if path.exists() {
            let raw = fs::read_to_string(&path)?;
            serde_json::from_str::<DocsData>(&raw)?
        } else {
            DocsData::default()
        };
        let docs = data
            .documents
            .into_iter()
            .map(|d| (d.id.clone(), d))
            .collect();
        Ok(Self { path, docs })
    }

    pub fn save(&self) -> anyhow::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = DocsData {
            documents: self.docs.values().cloned().collect(),
        };
        let json = serde_json::to_string_pretty(&data)?;
        fs::write(&self.path, json)?;
        Ok(())
    }

    /// Ingest a document, splitting it into chunks.
    pub fn ingest(
        &mut self,
        doc_id: &str,
        title: &str,
        source: &str,
        raw_content: &str,
    ) -> &Document {
        let chunks = chunk_text(doc_id, raw_content, CHUNK_SIZE, CHUNK_OVERLAP);
        let doc = Document {
            id: doc_id.to_string(),
            title: title.to_string(),
            source: source.to_string(),
            raw_content: raw_content.to_string(),
            chunks,
        };
        self.docs.insert(doc_id.to_string(), doc);
        self.docs.get(doc_id).unwrap()
    }

    pub fn get(&self, doc_id: &str) -> Option<&Document> {
        self.docs.get(doc_id)
    }

    pub fn list(&self) -> impl Iterator<Item = &Document> {
        self.docs.values()
    }

    /// Search chunks across all documents by keyword.
    pub fn search_chunks(&self, query: &str) -> Vec<&Chunk> {
        let q = query.to_lowercase();
        self.docs
            .values()
            .flat_map(|d| &d.chunks)
            .filter(|c| c.text.to_lowercase().contains(&q))
            .collect()
    }

    pub fn doc_count(&self) -> usize {
        self.docs.len()
    }
}

fn chunk_text(doc_id: &str, text: &str, size: usize, overlap: usize) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut offset = 0;
    while offset < chars.len() {
        let end = (offset + size).min(chars.len());
        let slice: String = chars[offset..end].iter().collect();
        chunks.push(Chunk {
            id: Uuid::new_v4(),
            doc_id: doc_id.to_string(),
            text: slice,
            offset,
        });
        if end == chars.len() {
            break;
        }
        offset += size - overlap;
    }
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunking_basic() {
        let chunks = chunk_text("d1", "abcdefghij", 4, 1);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].text, "abcd");
        assert_eq!(chunks[0].offset, 0);
        assert_eq!(chunks[1].text, "defg");
        assert_eq!(chunks[1].offset, 3);
    }

    #[test]
    fn ingest_and_search() {
        let mut store = DocumentStore::open("/tmp/kgx_test_docs.json").unwrap();
        store.ingest("d1", "Test", "test.md", "memory leak causes crash");
        let results = store.search_chunks("memory");
        assert!(!results.is_empty());
    }
}
