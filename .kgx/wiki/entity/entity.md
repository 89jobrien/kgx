---
title: Entity
type: type
source_document: kgx-codebase
tags: [entity, type]
---

# Entity

A node in the knowledge graph, stored by [[GraphStore]].

## Fields

- `id: NodeId` (UUID)
- `name: String`
- `entity_type: String`
- `supporting_text: Option<String>`
- `source_docs: Vec<DocId>` — provenance links to [[Document]]s

## Deduplication

Nodes are deduplicated by lowercased name. Adding the same entity
again merges source_docs rather than creating a duplicate.

## Connected by

[[Relation]] edges link entities directionally with confidence scores.
