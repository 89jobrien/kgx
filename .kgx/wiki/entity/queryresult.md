---
title: QueryResult
type: type
source_document: kgx-codebase
tags: [entity, type]
---

# QueryResult

Returned by `kgx query <name>`. Contains the BFS-extracted subgraph
plus provenance data.

## Fields

- `query: String` — the seed entity name
- `nodes: Vec<Entity>` — [[Entity]] nodes within traversal limits
- `edges: Vec<Relation>` — [[Relation]] edges within traversal limits
- `supporting_chunks: Vec<Chunk>` — [[Chunk]]s from source [[Document]]s

## Constraints

Respects MAX_GRAPH_DEPTH (2), MIN_CONFIDENCE (0.6), MAX_NODES (50).
