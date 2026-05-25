---
title: GraphStore
type: component
source_document: kgx-codebase
tags: [entity, core]
---

# GraphStore

Persistent entity-relation graph backed by a JSON file (`data/graph.json`).

## Responsibilities

- Store [[Entity]] nodes in a HashMap, deduplicated by lowercased name
- Store [[Relation]] edges in a Vec
- [[BFS-traversal]] from a seed node with depth/confidence/node limits
- Keyword search across node names and supporting text
- Persist to [[graph-json]]

## Key Methods

- `open(path)` — load or create from JSON file
- `add_node(name, type, text, doc)` — add/dedup entity, merge source docs
- `add_edge(EdgeInput)` — add relation if confidence >= MIN_CONFIDENCE
- `bfs_subgraph(seed)` — BFS extraction respecting constraints
- `search(query)` — keyword search on nodes
- `save()` — write sorted nodes + edges to JSON

## Internal Index

`name_index: HashMap<String, NodeId>` maps lowercased names to UUIDs
for O(1) dedup and lookup.
