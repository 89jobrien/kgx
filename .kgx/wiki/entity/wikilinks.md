---
title: wikilinks
type: concept
source_document: kgx-codebase
tags: [entity, concept]
---

# Wikilinks

Double-bracket syntax used in wiki markdown pages for linking between pages.
Parsed by [[wikilink-parser]], link text is normalized via [[slugify]].
Used by [[WikiStore]] lint to detect broken links and orphan pages.
