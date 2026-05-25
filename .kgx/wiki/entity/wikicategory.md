---
title: WikiCategory
type: type
source_document: kgx-codebase
tags: [entity, type]
---

# WikiCategory

Enum controlling wiki page organization in [[WikiStore]].

## Variants

- `Summary` → `wiki/summary/`
- `Entity` → `wiki/entity/`
- `Topic` → `wiki/topic/`

## Parsing

Implements `FromStr` and `Display` for roundtrip conversion.
Rejects unknown category strings with [[WikiCategoryParseError]].
