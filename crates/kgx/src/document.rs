use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::types::{Chunk, DocId, Document};
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

// qual:allow(iosp) reason: "I/O boundary — must check existence then read"
fn read_docs_data(path: &std::path::Path) -> anyhow::Result<DocsData> {
    if path.exists() {
        let raw = fs::read_to_string(path)?;
        Ok(serde_json::from_str::<DocsData>(&raw)?)
    } else {
        Ok(DocsData::default())
    }
}

impl DocumentStore {
    pub fn open(path: impl Into<PathBuf>) -> anyhow::Result<Self> {
        let path = path.into();
        let data = read_docs_data(&path)?;
        Ok(Self::from_data(path, data))
    }

    fn from_data(path: PathBuf, data: DocsData) -> Self {
        let docs = data
            .documents
            .into_iter()
            .map(|d| (d.id.clone(), d))
            .collect();
        Self { path, docs }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut documents: Vec<_> = self.docs.values().cloned().collect();
        documents.sort_by(|a, b| a.id.cmp(&b.id));
        let data = DocsData { documents };
        let json = serde_json::to_string_pretty(&data)?;
        fs::write(&self.path, json)?;
        Ok(())
    }

    /// Ingest a document, splitting it into chunks.
    /// Replaces any existing document with the same `doc_id`.
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
        self.docs
            .entry(doc_id.to_string())
            .insert_entry(doc)
            .into_mut()
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
    use proptest::prelude::*;

    fn fresh_store() -> DocumentStore {
        let path = format!("/tmp/kgx_test_docs_{}.json", Uuid::new_v4());
        DocumentStore::open(path).expect("fresh store should open")
    }

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
    fn chunking_empty_input() {
        let chunks = chunk_text("d1", "", 100, 10);
        assert!(chunks.is_empty());
    }

    #[test]
    fn chunking_exact_size() {
        let chunks = chunk_text("d1", "abcd", 4, 1);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, "abcd");
    }

    #[test]
    fn chunking_smaller_than_chunk_size() {
        let chunks = chunk_text("d1", "ab", 10, 2);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, "ab");
    }

    #[test]
    fn ingest_and_search() {
        let mut store = fresh_store();
        store.ingest("d1", "Test", "test.md", "memory leak causes crash");
        let results = store.search_chunks("memory");
        assert!(!results.is_empty());
    }

    #[test]
    fn save_and_reload_roundtrip() {
        let path = format!("/tmp/kgx_test_docs_rt_{}.json", Uuid::new_v4());
        {
            let mut store = DocumentStore::open(&path).expect("open should work");
            store.ingest("d1", "Doc One", "a.md", "hello world");
            store.ingest("d2", "Doc Two", "b.md", "goodbye world");
            store.save().expect("save should work");
        }
        let store2 = DocumentStore::open(&path).expect("reopen should work");
        assert_eq!(store2.doc_count(), 2);
        assert!(store2.get("d1").is_some());
        assert!(store2.get("d2").is_some());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn list_returns_all_docs() {
        let mut store = fresh_store();
        store.ingest("a", "A", "a.md", "aaa");
        store.ingest("b", "B", "b.md", "bbb");
        let ids: Vec<&str> = store.list().map(|d| d.id.as_str()).collect();
        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn ingest_replaces_existing_doc() {
        let mut store = fresh_store();
        store.ingest("d1", "First", "a.md", "original content");
        store.ingest("d1", "Second", "b.md", "replacement content");
        assert_eq!(store.doc_count(), 1);
        let doc = store.get("d1").expect("doc should exist");
        assert_eq!(doc.title, "Second");
    }

    // Property: every char in the input appears in at least one chunk.
    proptest! {
        #[test]
        fn chunking_covers_all_input(
            text in ".{1,200}",
            size in 1usize..50,
        ) {
            let overlap = size / 2;
            let chunks = chunk_text("d", &text, size, overlap);
            let reassembled: String = {
                let mut chars = std::collections::BTreeSet::new();
                for chunk in &chunks {
                    for (i, c) in chunk.text.chars().enumerate() {
                        chars.insert((chunk.offset + i, c));
                    }
                }
                let input_chars: Vec<char> = text.chars().collect();
                for (i, c) in input_chars.iter().enumerate() {
                    prop_assert!(
                        chars.contains(&(i, *c)),
                        "char at offset {} not covered by any chunk", i
                    );
                }
                String::new() // just for type
            };
            let _ = reassembled;
        }
    }
}
