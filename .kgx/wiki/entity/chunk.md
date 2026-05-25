---
title: Chunk
type: type
source_document: kgx-codebase
tags: [entity, type]
---

# Chunk

A slice of a [[Document]], used for provenance in [[QueryResult]].

## Fields

- `id: ChunkId` (UUID)
- `doc_id: DocId`
- `text: String`
- `offset: usize` — character offset in the original document

## Creation

Produced by [[chunking]] during [[DocumentStore]] ingest.
Default: 1000 chars with 200-char overlap between adjacent chunks.
