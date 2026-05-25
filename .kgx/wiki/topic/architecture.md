---
title: Architecture
source_document: kgx-codebase
tags: [topic, architecture]
---

# kgx Architecture

## Design Principles

1. **Three independent layers** — graph, documents, wiki — each with its
   own store and persistence format
2. **JSON-only storage** — no external database, all data as plain files
3. **Provenance** — every graph node links back to source documents via
   doc_id; chunks enable fine-grained citation
4. **Cross-references** — wiki pages use `[[wikilinks]]` for navigability

## Data Flow

```
Source Document
  |
  v
[ingest pipeline]
  |
  +---> [[DocumentStore]] (raw content + chunks)
  |
  +---> [[GraphStore]] (entities + relations)
  |
  v
[wiki write]
  |
  +---> [[WikiStore]] (markdown pages with wikilinks)
```

## Query Flow

```
Query (entity name)
  |
  v
[[GraphStore]].bfs_subgraph(seed)
  |
  +---> nodes + edges (within depth/confidence/node limits)
  |
  v
[[DocumentStore]].get(doc_ids)
  |
  +---> supporting chunks for provenance
  |
  v
[[QueryResult]] (nodes + edges + chunks)
```

## Export

[[ExportContext]] bundles all three stores. The [[Exporter-trait]]
allows format-agnostic export:

- [[JsonExporter]] — single `kgx-export.json`
- [[MarkdownExporter]] — Obsidian-compatible vault with frontmatter

## Crate Structure

- `kgx` (library): types, graph, document, wiki, export, ingest, init
- `kgx-cli` (binary): CLI parsing (clap), stdin I/O, command dispatch
