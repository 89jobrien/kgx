---
title: BFS traversal
type: algorithm
source_document: kgx-codebase
tags: [entity, algorithm]
---

# BFS Traversal

Implemented in [[GraphStore]]`.bfs_subgraph(seed)`.

## Algorithm

1. Start from seed node at depth 0
2. Visit neighbors via [[Relation]] edges (both directions)
3. Skip edges with confidence < MIN_CONFIDENCE (0.6)
4. Stop expanding at MAX_GRAPH_DEPTH (2)
5. Stop collecting at MAX_NODES (50)
6. Deduplicate edges using EdgeId HashSet

## Output

Returns `(Vec<Entity>, Vec<Relation>)` — the extracted subgraph.
Fed into [[QueryResult]] with supporting [[Chunk]]s.
