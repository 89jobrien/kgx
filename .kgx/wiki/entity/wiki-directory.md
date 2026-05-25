---
title: wiki directory
type: storage
source_document: kgx-codebase
tags: [entity, storage]
---

# wiki directory

Located at `wiki/` within the kgx workspace root. Managed by [[WikiStore]].

## Structure

```
wiki/
  summary/    # Document summaries
  entity/     # Entity reference pages
  topic/      # Synthesis and topic pages
  index.md    # Auto-generated index
  log.md      # Append-only write log
```

Each page is a markdown file named by [[slugify]](title).
Pages use `[[wikilinks]]` for cross-references.
