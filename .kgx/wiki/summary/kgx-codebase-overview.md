---
title: kgx Codebase Overview
source_document: kgx-codebase
tags: [summary, architecture]
---

# kgx Codebase Overview

**Source:** /Users/joe/dev/kgx

Three-layer knowledge graph toolkit in Rust with JSON-backed,
zero-dependency storage.

## Workspace Structure

Two crates:

- **[[kgx-library]]** — core library with all data structures and stores
- **[[kgx-cli]]** — CLI binary wrapping the library

## Three Layers

| Layer     | Store             | Storage            |
| --------- | ----------------- | ------------------ |
| Graph     | [[GraphStore]]    | [[graph-json]]     |
| Documents | [[DocumentStore]] | [[documents-json]] |
| Wiki      | [[WikiStore]]     | [[wiki-directory]] |

## Key Algorithms

- [[BFS-traversal]] — subgraph extraction with depth/confidence/node limits
- [[chunking]] — document splitting with overlap for provenance
- [[wikilink-parser]] — cross-reference extraction for lint

## Export

- [[JsonExporter]] — single JSON file
- [[MarkdownExporter]] — Obsidian-compatible vault

## Constraints

- MAX_GRAPH_DEPTH = 2
- MIN_CONFIDENCE = 0.6
- MAX_NODES = 50

## Quality

100% rustqual score across all six dimensions.

## License

MIT OR Apache-2.0
