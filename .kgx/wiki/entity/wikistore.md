---
title: WikiStore
type: component
source_document: kgx-codebase
tags: [entity, core]
---

# WikiStore

Manages a directory of markdown wiki pages organized by [[WikiCategory]]
(summary, entity, topic).

## Responsibilities

- Write/read markdown pages as files under `wiki/{category}/{slug}.md`
- [[wikilink-parser]] for [[wikilinks]] between pages
- Keyword search across all pages
- [[LintReport]] generation: orphan pages, missing pages, broken wikilinks
- Auto-maintain `index.md` and `log.md`

## Key Methods

- `open(root)` — create category dirs and open
- `write_page(cat, title, content, summary)` — write/overwrite, update index
- `read_page(cat, title)` — read by category + title
- `search(query)` — keyword search with snippets
- `list_pages(cat)` — sorted slugs in a category
- `lint()` — health check returning [[LintReport]]
- `pages()` — iterate all pages as [[WikiPage]] values

## Slug Generation

Uses [[slugify]]: lowercase, replace non-alphanumeric with hyphens,
collapse consecutive hyphens. Property-tested for idempotence.
