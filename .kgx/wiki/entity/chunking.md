---
title: chunking
type: algorithm
source_document: kgx-codebase
tags: [entity, algorithm]
---

# Chunking

Splits raw document text into overlapping [[Chunk]]s.

## Parameters

- `CHUNK_SIZE = 1000` characters
- `CHUNK_OVERLAP = 200` characters

## Properties (verified by proptest)

- Every character in the input appears in at least one chunk
- Chunks are contiguous with consistent offsets
