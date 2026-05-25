---
title: MarkdownExporter
type: component
source_document: kgx-codebase
tags: [entity, export]
---

# MarkdownExporter

Implements [[Exporter-trait]]. Creates an Obsidian-compatible vault:

- `entities/` — one `.md` per [[Entity]] with YAML frontmatter, [[Relation]]
  links, inlined source [[Chunk]]s
- `documents/` — one `.md` per [[Document]] with chunk boundaries
- `wiki/` — mirrors [[WikiStore]] category structure with backlinks
- `index.md` — stats and links to all pages
