# kgx

Three-layer knowledge graph toolkit in Rust.

## Build & Test

```bash
cargo build
cargo test
cargo clippy
```

## Architecture

| Module     | Purpose                                                     |
| ---------- | ----------------------------------------------------------- |
| `types`    | Core types: Entity, Relation, Document, Chunk, WikiPage     |
| `graph`    | Entity-relation graph with JSON persistence, BFS traversal  |
| `document` | Raw document store with chunking and provenance             |
| `wiki`     | Markdown wiki pages with wikilink cross-references and lint |

## Storage

All persistence is JSON files — no external DB. The three data files:

- `data/graph.json` — entities and relations
- `data/documents.json` — raw documents and chunks
- `wiki/` — markdown pages organized by category (summary, entity, topic)

## Constraints

- `MAX_GRAPH_DEPTH = 2` — BFS traversal limit
- `MIN_CONFIDENCE = 0.6` — edges below this are rejected
- `MAX_NODES = 50` — max nodes returned per query
