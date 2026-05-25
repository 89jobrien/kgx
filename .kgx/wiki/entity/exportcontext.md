---
title: ExportContext
type: component
source_document: kgx-codebase
tags: [entity, export]
---

# ExportContext

Bundles references to all three stores for export.

## Fields

- `graph: &GraphStore` — [[GraphStore]]
- `docs: &DocumentStore` — [[DocumentStore]]
- `wiki: &WikiStore` — [[WikiStore]]

## Usage

Passed to [[Exporter-trait]] implementations ([[JsonExporter]],
[[MarkdownExporter]]).
