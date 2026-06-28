# kgx — Agent Operating Guide

## Overview

kgx is a three-layer knowledge graph toolkit for building, querying, and
maintaining structured knowledge. It combines raw document storage, entity-
relation graphs, and markdown wiki pages into a unified persistence model.
All data is stored as JSON files (no external database).

## Quick Reference

| Command             | Purpose                         |
| ------------------- | ------------------------------- |
| `cargo build`       | Build kgx library and CLI       |
| `cargo test`        | Run all unit tests              |
| `cargo clippy`      | Lint code (strict, -D warnings) |
| `cargo nextest run` | Faster parallel test runner     |

## Build & Development

### Initial Setup

```bash
cd /Users/joe/dev/kgx
cargo build          # Build all crates (kgx, kgx-cli)
cargo test           # Run tests
cargo clippy -- -D warnings  # Lint
```

### Individual Crate Commands

```bash
# Test library only
cd crates/kgx && cargo test

# Test CLI only
cd crates/kgx-cli && cargo test

# Run CLI
cargo run -p kgx-cli -- --help

# Build release binary
cargo build --release -p kgx-cli
```

## Code Style & Conventions

### Formatting

- **Max width**: 100 characters
- **Edition**: 2024
- **Tool**: `cargo fmt --all`

Run before every commit:

```bash
cargo fmt --all --check    # Verify formatting
cargo fmt --all             # Auto-fix formatting
```

### Linting (Clippy)

Strict linting enforced:

```bash
cargo clippy --all-targets -- -D warnings
```

**Disallowed patterns** (test safety):

- `std::env::temp_dir()` → Use test utilities for isolated temp dirs
- `std::env::set_var()` / `remove_var()` → Use injection instead
- `std::thread::sleep()` → Use polling or proper test fixtures

### Naming Conventions

| Type              | Style                | Example                              |
| ----------------- | -------------------- | ------------------------------------ |
| Structs/Enums     | PascalCase           | `Entity`, `Relation`, `GraphNode`    |
| Functions/Methods | snake_case           | `add_entity`, `traverse_graph`       |
| Constants         | SCREAMING_SNAKE_CASE | `MAX_GRAPH_DEPTH`, `MIN_CONFIDENCE`  |
| Modules           | snake_case           | `types`, `graph`, `document`, `wiki` |

### Error Handling

- Primary: `anyhow::Result<T>` for application errors
- Secondary: `thiserror::Error` for custom error types
- Never: `unwrap()`, `expect()` in library code
- Pattern: Propagate with `?` operator

## Project Structure

### Workspace Layout

```
kgx/
├── Cargo.toml                 # Workspace root (2 members)
├── crates/
│   ├── kgx/                  # Library (core types & algorithms)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs        # Exports: types, graph, document, wiki
│   │       ├── types.rs      # Entity, Relation, Document, Chunk
│   │       ├── graph.rs      # Graph, JSON persistence, BFS
│   │       ├── document.rs   # Raw documents, chunking
│   │       └── wiki.rs       # Wiki pages, linting
│   └── kgx-cli/              # CLI binary (clap-based)
│       ├── Cargo.toml
│       └── src/
│           └── main.rs       # CLI entry point
├── docs/                      # Architecture & design docs
└── mise.toml                  # Task runner config
```

### Core Modules

| Module     | Exports                                               | Purpose                              |
| ---------- | ----------------------------------------------------- | ------------------------------------ |
| `types`    | `Entity`, `Relation`, `Document`, `Chunk`, `WikiPage` | Core domain types with serde support |
| `graph`    | `Graph`, `GraphNode`, traversal methods               | Entity-relation graph with JSON I/O  |
| `document` | `DocumentStore`, chunking logic                       | Raw document persistence             |
| `wiki`     | `WikiBuilder`, link resolution                        | Markdown wiki with cross-references  |

## Storage Format

Three JSON data files (created on first use):

- `data/graph.json` — Entities and relations
- `data/documents.json` — Raw documents and chunks (provenance)
- `wiki/` — Markdown pages by category (summary, entity, topic)

## Algorithm Constraints

| Constant          | Value | Purpose                        |
| ----------------- | ----- | ------------------------------ |
| `MAX_GRAPH_DEPTH` | 2     | BFS traversal depth limit      |
| `MIN_CONFIDENCE`  | 0.6   | Confidence threshold for edges |
| `MAX_NODES`       | 50    | Max nodes returned per query   |

Adjust in `types.rs` if traversal behavior changes.

## Testing

### Test Organization

- **Unit tests**: In same file as implementation (`#[cfg(test)]`)
- **Integration tests**: In `tests/` directory (if present)
- **Property tests**: Use `proptest` for fuzzing entity/relation logic

### Running Tests

```bash
# Run all tests
cargo test --all

# Run single test function
cargo test test_graph_traversal

# Run with output
cargo test -- --nocapture

# Use nextest for speed
cargo nextest run --all

# Test one crate
cd crates/kgx && cargo test
```

### Test Isolation

- Use injected file paths (no hardcoded temp dirs)
- Pass `TempDir` to constructors that need file I/O
- Avoid global state; use local fixtures

## Dependencies

### Core

- `serde` / `serde_json` — JSON serialization
- `serde_yaml` — YAML parsing (for wiki frontmatter)
- `thiserror` — Structured error types
- `anyhow` — Error propagation

### CLI

- `clap` — Command-line argument parsing (derive macros)

### Testing

- `proptest` — Property-based testing
- `assert_cmd` — CLI integration test helpers

## Commit Guidelines

Follow conventional commits:

```
<type>(<scope>): <description>

Examples:
  feat(graph): add bidirectional traversal
  fix(wiki): resolve circular wikilink detection
  docs(types): explain Entity relation cardinality
  refactor(document): simplify chunk batching logic
  test(graph): add depth-limit property tests
```

Valid types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`

## Development Workflow

1. **Create feature branch**: `git checkout -b feature/description`
2. **Implement & test**: `cargo test --all`
3. **Format & lint**: `cargo fmt --all && cargo clippy -- -D warnings`
4. **Commit**: Use conventional format
5. **Push & PR**: Await review
6. **Merge**: Squash or rebase as appropriate

## Common Tasks

### Adding a New Type

1. Define in `types.rs` with `#[derive(Serialize, Deserialize)]`
2. Add tests in same file (`#[cfg(test)]` module)
3. Export from `lib.rs`
4. Update relevant module docs

### Extending Graph Traversal

1. Add method to `graph.rs` (e.g., `traverse_depth_first`)
2. Respect `MAX_GRAPH_DEPTH` and `MAX_NODES` constraints
3. Add property tests with `proptest`
4. Document complexity in docstring

### CLI Command Integration

1. Add subcommand to kgx-cli `main.rs`
2. Use `clap` derive macros for arg parsing
3. Return `anyhow::Result<()>` from handler
4. Test with `assert_cmd` integration tests

## CI/CD

Pre-commit checks (run before every commit):

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test --all
```

Equivalent to: `just ci` (if justfile configured)

## Key Dependencies & Versions

See `Cargo.toml` for pinned versions. Workspace uses shared version
specification to keep all crates in sync (e.g., `version.workspace = true`).

### Update Procedure

```bash
cargo update                  # Update minor/patch versions
cargo update -p <crate-name>  # Update specific crate
cargo outdated                # Check for newer versions
```

Always commit `Cargo.lock` when dependencies change.

## Resources

- **README**: `/Users/joe/dev/kgx/README.md` — High-level overview
- **CLAUDE.md**: `/Users/joe/dev/kgx/CLAUDE.md` — Project config
- **Architecture**: `/Users/joe/dev/kgx/docs/` — Design docs
