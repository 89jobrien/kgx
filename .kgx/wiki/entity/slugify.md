---
title: slugify
type: function
source_document: kgx-codebase
tags: [entity, function]
---

# slugify

`pub fn slugify(title: &str) -> String`

Converts a title string to a URL-safe slug for wiki page filenames.

## Rules

1. Lowercase the input
2. Replace non-alphanumeric chars with hyphens
3. Collapse consecutive hyphens
4. Strip leading/trailing hyphens

## Properties (verified by proptest)

- Idempotent: `slugify(slugify(x)) == slugify(x)`
- No consecutive hyphens in output

## Used By

[[WikiStore]] for page file naming and [[wikilink-parser]] for
normalizing link targets.
