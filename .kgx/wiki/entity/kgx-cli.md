---
title: kgx-cli
type: crate
source_document: kgx-codebase
tags: [entity, crate]
---

# kgx-cli

CLI binary crate. Depends on [[kgx-library]].

## Subcommands

- `init` — bootstrap a new workspace
- `ingest` — ingest JSON from stdin (document + entities + relations)
- `query <name>` — [[BFS-traversal]] from a seed entity
- `graph add-node / add-edge / search` — direct [[GraphStore]] ops
- `wiki write / read / search / list / lint` — [[WikiStore]] ops
- `docs list / search` — [[DocumentStore]] ops
- `export --format json|markdown --output <dir>` — full context export
- `stats` — node/edge/document counts

## Global Flag

`--root <dir>` — data directory (default: `.kgx` in cwd)
