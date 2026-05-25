---
title: DocumentStore
type: component
source_document: kgx-codebase
tags: [entity, core]
---

# DocumentStore

Persistent raw document store backed by `data/documents.json`.

## Responsibilities

- Store [[Document]] objects in a HashMap by doc_id
- Split ingested content into overlapping [[Chunk]]s (1000 chars, 200 overlap)
- Keyword search across all chunks
- Persist to [[documents-json]]

## Key Methods

- `open(path)` — load or create from JSON file
- `ingest(doc_id, title, source, content)` — store and chunk a document
- `get(doc_id)` — retrieve a document
- `search_chunks(query)` — keyword search across all chunks
- `save()` — write to JSON

## Chunking

Uses `chunk_text()` with configurable size and overlap.
Property-tested: every char in input appears in at least one chunk.
