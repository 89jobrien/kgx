---
title: graph.json
type: storage
source_document: kgx-codebase
tags: [entity, storage]
---

# graph.json

Located at `data/graph.json` within the kgx workspace root.

## Format

```json
{
  "nodes": [Entity...],
  "edges": [Relation...]
}
```

Nodes are sorted by name on save. Managed by [[GraphStore]].
