---
title: Relation
type: type
source_document: kgx-codebase
tags: [entity, type]
---

# Relation

A directed edge between two [[Entity]] nodes in [[GraphStore]].

## Fields

- `id: EdgeId` (UUID)
- `source: NodeId`
- `target: NodeId`
- `relation_type: String`
- `confidence: f64` — must be >= MIN_CONFIDENCE (0.6) to be accepted
- `supporting_text: Option<String>`
- `source_doc: Option<DocId>`

## Confidence Filter

Edges below MIN_CONFIDENCE are silently rejected by `add_edge`.
[[BFS-traversal]] also skips low-confidence edges during neighbor iteration.
