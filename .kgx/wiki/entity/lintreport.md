---
title: LintReport
type: type
source_document: kgx-codebase
tags: [entity, type]
---

# LintReport

Returned by [[WikiStore]]`.lint()`.

## Fields

- `orphan_pages: Vec<String>` — pages never referenced by any wikilink
- `missing_pages: Vec<String>` — slugs referenced but no page exists
- `broken_wikilinks: Vec<(String, String)>` — (source_page, target_slug)
- `isolated_pages: Vec<String>` — pages with zero outgoing wikilinks
