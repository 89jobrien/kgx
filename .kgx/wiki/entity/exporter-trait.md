---
title: Exporter trait
type: interface
source_document: kgx-codebase
tags: [entity, export]
---

# Exporter Trait

```rust
pub trait Exporter {
    fn export(&self, ctx: &ExportContext, output: &Path) -> Result<()>;
}
```

## Implementations

- [[JsonExporter]] — exports as single JSON file
- [[MarkdownExporter]] — exports as Obsidian-compatible vault

## Input

Takes an [[ExportContext]] bundling [[GraphStore]], [[DocumentStore]],
and [[WikiStore]].
