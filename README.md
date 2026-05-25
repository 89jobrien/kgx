# kgx

Three-layer knowledge graph toolkit in Rust.

JSON-backed, zero-dependency storage. No external database required --
entities, relations, documents, and wiki pages all persist as plain files.

## Architecture

```
                    +-----------------+
                    |    kgx-cli      |  CLI binary
                    +--------+--------+
                             |
          +------------------+------------------+
          |                  |                  |
  +-------+------+  +-------+-------+  +-------+------+
  | GraphStore   |  | DocumentStore |  | WikiStore    |
  | graph.json   |  | documents.json|  | wiki/        |
  +--------------+  +---------------+  +--------------+
   BFS traversal       Chunking &       Markdown pages
   Confidence filter    Provenance       [[wikilinks]]
```

| Layer         | Storage               | Purpose                                             |
| ------------- | --------------------- | --------------------------------------------------- |
| **Graph**     | `data/graph.json`     | Entity-relation graph, BFS traversal, confidence    |
| **Documents** | `data/documents.json` | Immutable document store with chunking & provenance |
| **Wiki**      | `wiki/`               | Markdown pages with `[[wikilinks]]`, search, lint   |

## Quick Start


### Initialize a workspace
```bash
kgx --root ./my-kb init
```
### Add entities and relations

```bash
kgx --root ./my-kb graph add-node "Rust" --type language
kgx --root ./my-kb graph add-node "Memory Safety" --type concept
kgx --root ./my-kb graph add-edge "Rust" "Memory Safety" \
    --type enables --confidence 0.95
```
### Ingest a document with entities and relations
```bash
kgx --root ./my-kb ingest --file notes.json
```
### Search the graph
```bash
kgx --root ./my-kb graph search "Rust"
```

### Write and search wiki pages
```
kgx --root ./my-kb wiki write --category entity --title "Rust" \
    --summary "A systems language" < rust.md
kgx --root ./my-kb wiki search "memory"
```
### Lint for broken wikilinks
```bash
kgx --root ./my-kb wiki lint
```
### Workspace stats
```bash
kgx --root ./my-kb stats
```

## Library Usage

```rust
use kgx::{GraphStore, DocumentStore, WikiStore, WikiCategory};

let mut graph = GraphStore::open("data/graph.json")?;
let mut docs = DocumentStore::open("data/documents.json")?;
let wiki = WikiStore::open("wiki/")?;

// Build the graph
let leak = graph.add_node("memory leak", "issue",
    Some("causes crashes"), Some("doc_001"));
let crash = graph.add_node("system crash", "issue",
    Some("system crashes"), Some("doc_001"));
graph.add_edge(leak, crash, "causes", 1.0,
    Some("crashes due to memory leaks"), Some("doc_001"));

// BFS traversal
let (nodes, edges) = graph.bfs_subgraph(leak);

// Wiki with wikilinks and lint
wiki.write_page(WikiCategory::Summary, "Incident Report",
    "# Report\n\n[[memory-leak]] causes [[system-crash]].",
    "Summary of the incident.")?;
let report = wiki.lint()?;

graph.save()?;
docs.save()?;
```

## Retrieval Constraints

| Constant          | Value | Purpose                       |
| ----------------- | ----- | ----------------------------- |
| `MAX_GRAPH_DEPTH` | 2     | BFS traversal limit           |
| `MIN_CONFIDENCE`  | 0.6   | Edges below this are rejected |
| `MAX_NODES`       | 50    | Max nodes returned per query  |

## Code Quality

100% [rustqual](https://github.com/89jobrien/checkup) score across all
six dimensions: IOSP, Complexity, DRY, SRP, Coupling, Test Quality.

## License

MIT OR Apache-2.0
