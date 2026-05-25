---
title: kgx library
type: crate
source_document: kgx-codebase
tags: [entity, crate]
---

# kgx library

Core library crate exposing all data structures and stores.

## Modules

- `types` — [[Entity]], [[Relation]], [[Document]], [[Chunk]], [[WikiPage]], [[WikiCategory]], [[QueryResult]], [[LintReport]]
- `graph` — [[GraphStore]], [[BFS-traversal]]
- `document` — [[DocumentStore]], [[chunking]]
- `wiki` — [[WikiStore]], [[slugify]], [[wikilink-parser]]
- `export` — [[ExportContext]], [[Exporter-trait]], [[JsonExporter]], [[MarkdownExporter]]
- `ingest` — ingest_entities, ingest_relations
- `init` — init_workspace

## Path Helpers

- `graph_path(root)` → `root/data/graph.json`
- `docs_path(root)` → `root/data/documents.json`
- `wiki_path(root)` → `root/wiki`
