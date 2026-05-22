# kgx

Three-layer knowledge graph toolkit in Rust.

## Layers

| Layer           | Storage               | Purpose                                                           |
| --------------- | --------------------- | ----------------------------------------------------------------- |
| **Raw Sources** | `data/documents.json` | Immutable document store with chunking and provenance             |
| **Graph**       | `data/graph.json`     | Entity-relation graph with BFS traversal and confidence filtering |
| **Wiki**        | `wiki/`               | Markdown pages with `[[wikilinks]]`, search, and lint             |

## Usage

```rust
use kgx::{GraphStore, DocumentStore, WikiStore, WikiCategory};

// Open stores (creates files if they don't exist)
let mut graph = GraphStore::open("data/graph.json")?;
let mut docs = DocumentStore::open("data/documents.json")?;
let wiki = WikiStore::open("wiki/")?;

// Ingest a document
docs.ingest("doc_001", "Incident Report", "report.md",
    "System crashes due to memory leaks.");

// Build the graph
let leak = graph.add_node("memory leak", "issue",
    Some("memory leaks cause crashes"), Some("doc_001"));
let crash = graph.add_node("system crash", "issue",
    Some("system crashes due to memory leaks"), Some("doc_001"));
graph.add_edge(leak, crash, "causes", 1.0, Some("crashes due to memory leaks"), Some("doc_001"));

// Query via BFS
let (nodes, edges) = graph.bfs_subgraph(leak);

// Write a wiki page
wiki.write_page(WikiCategory::Summary, "Incident Report",
    "# Incident Report\n\n[[memory-leak]] causes [[system-crash]].",
    "Memory leaks cause system crashes.")?;

// Search the wiki
let hits = wiki.search("memory")?;

// Lint for broken links and orphan pages
let report = wiki.lint()?;

// Persist
graph.save()?;
docs.save()?;
```

## Retrieval Constraints

- **MAX_GRAPH_DEPTH** = 2 — BFS traversal limit
- **MIN_CONFIDENCE** = 0.6 — edges below this threshold are rejected
- **MAX_NODES** = 50 — maximum nodes returned per query

## License

MIT OR Apache-2.0
