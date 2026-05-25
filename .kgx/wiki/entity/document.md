---
title: Document
type: type
source_document: kgx-codebase
tags: [entity, type]
---

# Document

A raw ingested document stored by [[DocumentStore]].

## Fields

- `id: DocId` (String)
- `title: String`
- `source: String` — original file path or URL
- `raw_content: String`
- `chunks: Vec<Chunk>` — overlapping [[Chunk]]s for provenance

## Lifecycle

Ingesting a document with an existing `doc_id` replaces the previous version.
