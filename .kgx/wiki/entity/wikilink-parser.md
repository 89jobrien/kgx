---
title: wikilink parser
type: algorithm
source_document: kgx-codebase
tags: [entity, algorithm]
---

# Wikilink Parser

`extract_wikilinks(content)` in the wiki module.

## Behavior

- Finds all double-bracket link patterns in markdown
- Runs each link through [[slugify]]
- Returns `Vec<String>` of slugified link texts
- Handles unclosed brackets gracefully (stops parsing)

## Used By

- [[WikiStore]]`.lint()` — to detect broken [[wikilinks]] and orphan pages
- [[LintReport]] generation
