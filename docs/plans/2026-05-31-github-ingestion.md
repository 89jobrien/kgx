# Plan: GitHub Repo Ingestion

## Goal

Ingest GitHub repositories into kgx as entities and relations via two
paths: a `GitHubParser` (stdin, raw `gh api` JSON) and a `GitHubSource`
(fetches via `gh api` subprocess). Layer 1 (metadata) is fully
implemented; layers 2-5 and alternative backends are stubbed.

## Architecture

- Crates affected: `kgx` (library), `kgx-cli` (binary)
- New traits/types: `Source` trait, `SourceError`, `Layer` enum,
  `GitHubParser`, `GitHubSource`
- Data flow: `gh api` JSON -> `GitHubParser::parse()` -> `ParsedDocument`
  -> existing `ingest_entities` / `ingest_relations` -> `GraphStore`

## Tech Stack

- Rust 2024 edition, no new dependencies
- `std::process::Command` for `gh` subprocess
- `serde_json` for GitHub API response deserialization

## Tasks

### Task 1: Source trait and module skeleton

**Crate**: `kgx`
**File(s)**: `crates/kgx/src/source/mod.rs`, `crates/kgx/src/lib.rs`
**Run**: `cargo check -p kgx`

1. Write failing test:

   ```rust
   // crates/kgx/src/source/mod.rs
   #[cfg(test)]
   mod tests {
       use super::*;
       use crate::parse::{ParsedDocument, ParsedEntity};

       struct FakeSource;

       impl Source for FakeSource {
           fn fetch(&self) -> Result<Vec<ParsedDocument>, SourceError> {
               Ok(vec![ParsedDocument {
                   doc_id: "fake".to_string(),
                   entities: vec![ParsedEntity {
                       name: "test".to_string(),
                       entity_type: "fake".to_string(),
                       supporting_text: None,
                   }],
                   relations: vec![],
               }])
           }
       }

       #[test]
       fn fake_source_returns_documents() {
           let docs = FakeSource.fetch().unwrap();
           assert_eq!(docs.len(), 1);
           assert_eq!(docs[0].doc_id, "fake");
       }
   }
   ```

   Run: `cargo nextest run -p kgx -- fake_source_returns_documents`
   Expected: FAIL (module does not exist)

2. Implement:

   ```rust
   // crates/kgx/src/source/mod.rs
   pub mod github;
   pub mod gitlab;
   pub mod local_git;

   use crate::parse::ParsedDocument;

   /// Errors from source fetching.
   #[derive(Debug, thiserror::Error)]
   pub enum SourceError {
       #[error("fetch failed: {0}")]
       FetchFailed(String),
       #[error("parse failed: {0}")]
       ParseFailed(#[from] crate::parse::ParseError),
       #[error("io error: {0}")]
       Io(#[from] std::io::Error),
   }

   /// A Source fetches data from an external system and yields parsed
   /// documents ready for ingestion.
   pub trait Source {
       fn fetch(&self) -> Result<Vec<ParsedDocument>, SourceError>;
   }
   ```

3. Register module in lib.rs -- add `pub mod source;` after `pub mod parse;`
   and add re-exports:

   ```rust
   pub use source::{Source, SourceError};
   ```

4. Verify:

   ```
   cargo nextest run -p kgx -- fake_source_returns_documents  -> green
   cargo clippy -p kgx -- -D warnings                         -> zero warnings
   ```

5. Commit: `feat(kgx): add Source trait and source module skeleton`

### Task 2: GitLab and LocalGit stubs

**Crate**: `kgx`
**File(s)**: `crates/kgx/src/source/gitlab.rs`,
`crates/kgx/src/source/local_git.rs`
**Run**: `cargo check -p kgx`

1. Write stubs:

   ```rust
   // crates/kgx/src/source/gitlab.rs
   /// GitLab repository source.
   ///
   /// Future implementation: fetch repo metadata, docs, and issues from
   /// the GitLab REST API and produce `ParsedDocument`s.
   pub struct GitLabSource {
       _private: (),
   }
   ```

   ```rust
   // crates/kgx/src/source/local_git.rs
   /// Local git repository source.
   ///
   /// Future implementation: walk a cloned git repo on disk, parse
   /// source files, README, and metadata into `ParsedDocument`s.
   pub struct LocalGitSource {
       _private: (),
   }
   ```

2. Verify:

   ```
   cargo check -p kgx  -> ok
   cargo clippy -p kgx -- -D warnings  -> zero warnings
   ```

3. Commit: `feat(kgx): add GitLabSource and LocalGitSource stubs`

### Task 3: AstParser stub

**Crate**: `kgx`
**File(s)**: `crates/kgx/src/parse/ast.rs`, `crates/kgx/src/parse/mod.rs`
**Run**: `cargo check -p kgx`

1. Write stub:

   ```rust
   // crates/kgx/src/parse/ast.rs
   /// AST-based code parser.
   ///
   /// Future implementation: use tree-sitter or syn to extract modules,
   /// functions, types, and imports from source files as entities and
   /// relations.
   pub struct AstParser {
       _private: (),
   }
   ```

2. Add `pub mod ast;` to `crates/kgx/src/parse/mod.rs`.

3. Verify:

   ```
   cargo check -p kgx  -> ok
   cargo clippy -p kgx -- -D warnings  -> zero warnings
   ```

4. Commit: `feat(kgx): add AstParser stub`

### Task 4: GitHubParser (Layer 1 -- metadata)

**Crate**: `kgx`
**File(s)**: `crates/kgx/src/parse/github.rs`, `crates/kgx/src/parse/mod.rs`
**Run**: `cargo nextest run -p kgx -- github`

1. Write failing tests:

   ```rust
   // crates/kgx/src/parse/github.rs
   #[cfg(test)]
   mod tests {
       use super::*;
       use crate::parse::{conformance, Parser};

       const SAMPLE_REPO: &str = r#"{
           "full_name": "rust-lang/rust",
           "description": "The Rust programming language",
           "owner": { "login": "rust-lang", "type": "Organization" },
           "topics": ["rust", "compiler"],
           "language": "Rust",
           "license": { "spdx_id": "MIT" }
       }"#;

       #[test]
       fn github_parser_extracts_repo_entity() {
           let p = GitHubParser;
           let doc = p.parse(SAMPLE_REPO, "gh-1").unwrap();
           let repo = doc.entities.iter().find(|e| e.entity_type == "repository");
           assert!(repo.is_some());
           assert_eq!(repo.unwrap().name, "rust-lang/rust");
       }

       #[test]
       fn github_parser_extracts_owner() {
           let p = GitHubParser;
           let doc = p.parse(SAMPLE_REPO, "gh-1").unwrap();
           let owner = doc.entities.iter().find(|e| e.name == "rust-lang");
           assert!(owner.is_some());
           assert_eq!(owner.unwrap().entity_type, "organization");
       }

       #[test]
       fn github_parser_extracts_topics() {
           let p = GitHubParser;
           let doc = p.parse(SAMPLE_REPO, "gh-1").unwrap();
           let topics: Vec<_> = doc.entities.iter()
               .filter(|e| e.entity_type == "topic")
               .map(|e| e.name.as_str())
               .collect();
           assert_eq!(topics, vec!["rust", "compiler"]);
       }

       #[test]
       fn github_parser_extracts_language() {
           let p = GitHubParser;
           let doc = p.parse(SAMPLE_REPO, "gh-1").unwrap();
           let lang = doc.entities.iter().find(|e| e.entity_type == "language");
           assert!(lang.is_some());
           assert_eq!(lang.unwrap().name, "Rust");
       }

       #[test]
       fn github_parser_extracts_license() {
           let p = GitHubParser;
           let doc = p.parse(SAMPLE_REPO, "gh-1").unwrap();
           let lic = doc.entities.iter().find(|e| e.entity_type == "license");
           assert!(lic.is_some());
           assert_eq!(lic.unwrap().name, "MIT");
       }

       #[test]
       fn github_parser_creates_relations() {
           let p = GitHubParser;
           let doc = p.parse(SAMPLE_REPO, "gh-1").unwrap();
           let rel_types: Vec<_> = doc.relations.iter()
               .map(|r| r.relation_type.as_str())
               .collect();
           assert!(rel_types.contains(&"owned_by"));
           assert!(rel_types.contains(&"tagged_with"));
           assert!(rel_types.contains(&"written_in"));
           assert!(rel_types.contains(&"licensed_under"));
       }

       #[test]
       fn github_parser_satisfies_contract() {
           conformance::assert_parser_contract(&GitHubParser, "github");
       }

       #[test]
       fn github_parser_handles_missing_optional_fields() {
           let json = r#"{
               "full_name": "user/minimal",
               "description": null,
               "owner": { "login": "user", "type": "User" }
           }"#;
           let p = GitHubParser;
           let doc = p.parse(json, "gh-2").unwrap();
           assert!(!doc.entities.is_empty());
       }
   }
   ```

   Run: `cargo nextest run -p kgx -- github_parser`
   Expected: FAIL

2. Implement:

   ```rust
   // crates/kgx/src/parse/github.rs
   use crate::parse::{ParseError, ParsedDocument, ParsedEntity, ParsedRelation, Parser};

   /// Parses raw GitHub API JSON (from `gh api repos/owner/repo`) into
   /// entities and relations.
   pub struct GitHubParser;

   /// Subset of GitHub API repo response we care about.
   #[derive(serde::Deserialize)]
   struct GitHubRepo {
       full_name: String,
       description: Option<String>,
       owner: GitHubOwner,
       #[serde(default)]
       topics: Vec<String>,
       language: Option<String>,
       license: Option<GitHubLicense>,
   }

   #[derive(serde::Deserialize)]
   struct GitHubOwner {
       login: String,
       #[serde(rename = "type")]
       owner_type: String,
   }

   #[derive(serde::Deserialize)]
   struct GitHubLicense {
       spdx_id: String,
   }

   impl Parser for GitHubParser {
       fn parse(&self, input: &str, doc_id: &str) -> Result<ParsedDocument, ParseError> {
           if input.trim().is_empty() {
               return Ok(ParsedDocument {
                   doc_id: doc_id.to_string(),
                   entities: vec![],
                   relations: vec![],
               });
           }

           let repo: GitHubRepo = serde_json::from_str(input)
               .map_err(|e| ParseError::Failed(e.to_string()))?;

           let mut entities = Vec::new();
           let mut relations = Vec::new();

           // Repo entity
           entities.push(ParsedEntity {
               name: repo.full_name.clone(),
               entity_type: "repository".to_string(),
               supporting_text: repo.description.clone(),
           });

           // Owner entity
           let owner_type = repo.owner.owner_type.to_lowercase();
           entities.push(ParsedEntity {
               name: repo.owner.login.clone(),
               entity_type: owner_type,
               supporting_text: None,
           });
           relations.push(ParsedRelation {
               source: repo.full_name.clone(),
               target: repo.owner.login.clone(),
               relation_type: "owned_by".to_string(),
               confidence: 1.0,
               supporting_text: None,
           });

           // Topics
           for topic in &repo.topics {
               entities.push(ParsedEntity {
                   name: topic.clone(),
                   entity_type: "topic".to_string(),
                   supporting_text: None,
               });
               relations.push(ParsedRelation {
                   source: repo.full_name.clone(),
                   target: topic.clone(),
                   relation_type: "tagged_with".to_string(),
                   confidence: 1.0,
                   supporting_text: None,
               });
           }

           // Language
           if let Some(lang) = &repo.language {
               entities.push(ParsedEntity {
                   name: lang.clone(),
                   entity_type: "language".to_string(),
                   supporting_text: None,
               });
               relations.push(ParsedRelation {
                   source: repo.full_name.clone(),
                   target: lang.clone(),
                   relation_type: "written_in".to_string(),
                   confidence: 1.0,
                   supporting_text: None,
               });
           }

           // License
           if let Some(lic) = &repo.license {
               entities.push(ParsedEntity {
                   name: lic.spdx_id.clone(),
                   entity_type: "license".to_string(),
                   supporting_text: None,
               });
               relations.push(ParsedRelation {
                   source: repo.full_name.clone(),
                   target: lic.spdx_id.clone(),
                   relation_type: "licensed_under".to_string(),
                   confidence: 1.0,
                   supporting_text: None,
               });
           }

           Ok(ParsedDocument {
               doc_id: doc_id.to_string(),
               entities,
               relations,
           })
       }
   }
   ```

3. Add `pub mod github;` to `crates/kgx/src/parse/mod.rs`.

4. Verify:

   ```
   cargo nextest run -p kgx -- github  -> all green
   cargo clippy -p kgx -- -D warnings  -> zero warnings
   ```

5. Commit: `feat(kgx): add GitHubParser for repo metadata extraction`

### Task 5: Layer enum and GitHubSource module

**Crate**: `kgx`
**File(s)**: `crates/kgx/src/source/github/mod.rs`,
`crates/kgx/src/source/github/http.rs`,
`crates/kgx/src/source/github/octocrab.rs`
**Run**: `cargo nextest run -p kgx -- github_source`

1. Write failing test:

   ```rust
   // crates/kgx/src/source/github/mod.rs
   #[cfg(test)]
   mod tests {
       use super::*;

       #[test]
       fn layer_ordering() {
           assert!(Layer::Metadata < Layer::Docs);
           assert!(Layer::Docs < Layer::Deps);
           assert!(Layer::Deps < Layer::Issues);
       }

       #[test]
       fn layer_includes_lower() {
           assert!(Layer::Docs.includes(Layer::Metadata));
           assert!(Layer::Issues.includes(Layer::Deps));
           assert!(!Layer::Metadata.includes(Layer::Docs));
       }
   }
   ```

   Run: `cargo nextest run -p kgx -- layer_ordering`
   Expected: FAIL

2. Implement:

   ```rust
   // crates/kgx/src/source/github/mod.rs
   pub mod http;
   pub mod octocrab;

   use std::process::Command;

   use crate::parse::github::GitHubParser;
   use crate::parse::{ParsedDocument, Parser};
   use crate::source::{Source, SourceError};

   /// Controls how much data to extract from a GitHub repo.
   #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
   pub enum Layer {
       Metadata,
       Docs,
       Deps,
       Issues,
   }

   impl Layer {
       /// Returns true if `self` includes the given layer.
       pub fn includes(self, other: Layer) -> bool {
           self >= other
       }
   }

   impl std::str::FromStr for Layer {
       type Err = String;

       fn from_str(s: &str) -> Result<Self, Self::Err> {
           match s {
               "metadata" => Ok(Layer::Metadata),
               "docs" => Ok(Layer::Docs),
               "deps" => Ok(Layer::Deps),
               "issues" => Ok(Layer::Issues),
               other => Err(format!(
                   "unknown layer: {other} (expected metadata, docs, deps, issues)"
               )),
           }
       }
   }

   /// Fetches GitHub repo data via the `gh` CLI.
   pub struct GitHubSource {
       owner_repo: String,
       layer: Layer,
   }

   impl GitHubSource {
       pub fn new(owner_repo: &str, layer: Layer) -> Self {
           Self {
               owner_repo: owner_repo.to_string(),
               layer,
           }
       }

       fn fetch_repo_json(&self) -> Result<String, SourceError> {
           let output = Command::new("gh")
               .args(["api", &format!("repos/{}", self.owner_repo)])
               .output()
               .map_err(|e| SourceError::FetchFailed(
                   format!("failed to run gh: {e}")
               ))?;

           if !output.status.success() {
               let stderr = String::from_utf8_lossy(&output.stderr);
               return Err(SourceError::FetchFailed(
                   format!("gh api failed: {stderr}")
               ));
           }

           String::from_utf8(output.stdout)
               .map_err(|e| SourceError::FetchFailed(
                   format!("invalid utf8: {e}")
               ))
       }
   }

   impl Source for GitHubSource {
       fn fetch(&self) -> Result<Vec<ParsedDocument>, SourceError> {
           let json = self.fetch_repo_json()?;
           let parser = GitHubParser;
           let doc_id = format!("github:{}", self.owner_repo);
           let doc = parser.parse(&json, &doc_id)?;
           // Layer 1 (Metadata) only for now.
           // Future: if self.layer.includes(Layer::Docs) { ... }
           Ok(vec![doc])
       }
   }
   ```

   ```rust
   // crates/kgx/src/source/github/http.rs
   /// HTTP-based GitHub API client using reqwest.
   ///
   /// Future implementation: direct REST calls without shelling out to
   /// `gh`. Requires `reqwest` and `tokio` dependencies.
   pub struct HttpGitHubSource {
       _private: (),
   }
   ```

   ```rust
   // crates/kgx/src/source/github/octocrab.rs
   /// Typed GitHub API client via the octocrab crate.
   ///
   /// Future implementation: ergonomic, typed API access with
   /// pagination and rate-limit handling built in.
   pub struct OctocrabGitHubSource {
       _private: (),
   }
   ```

3. Update `crates/kgx/src/source/mod.rs` to use `pub mod github;`
   (already declared in Task 1).

4. Verify:

   ```
   cargo nextest run -p kgx -- layer_  -> all green
   cargo clippy -p kgx -- -D warnings  -> zero warnings
   ```

5. Commit: `feat(kgx): add GitHubSource with Layer enum and backend stubs`

### Task 6: CLI -- add --format and --github flags to Ingest

**Crate**: `kgx-cli`
**File(s)**: `crates/kgx-cli/src/cli.rs`, `crates/kgx-cli/src/main.rs`
**Run**: `cargo nextest run -p kgx-cli`

1. Write failing test:

   ```rust
   // crates/kgx-cli/tests/cli.rs (append to existing)
   #[test]
   fn ingest_format_github_from_stdin() {
       let dir = tempdir().unwrap();
       init_workspace(dir.path());

       let json = r#"{
           "full_name": "test/repo",
           "description": "A test repo",
           "owner": { "login": "test", "type": "User" },
           "topics": ["kgx"],
           "language": "Rust",
           "license": { "spdx_id": "MIT" }
       }"#;

       let output = Command::cargo_bin("kgx")
           .unwrap()
           .args(["--root", dir.path().to_str().unwrap(), "ingest", "--format", "github"])
           .write_stdin(json)
           .assert()
           .success();

       let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
       assert!(stdout.contains("test/repo"));
   }
   ```

   Run: `cargo nextest run -p kgx-cli -- ingest_format_github`
   Expected: FAIL

2. Update CLI enum:

   ```rust
   // crates/kgx-cli/src/cli.rs — modify Ingest variant
   /// Ingest a document with entities and relations
   Ingest {
       /// Input format: json (default) or github
       #[arg(long, default_value = "json")]
       format: String,
       /// Fetch directly from GitHub (e.g. owner/repo)
       #[arg(long)]
       github: Option<String>,
       /// GitHub extraction layer: metadata, docs, deps, issues
       #[arg(long, default_value = "metadata")]
       github_layer: String,
   },
   ```

3. Update main.rs `cmd_ingest` to accept the new fields and dispatch:

   ```rust
   fn cmd_ingest(root: &Path, format: &str, github: Option<&str>, github_layer: &str) -> Result<()> {
       let mut graph = GraphStore::open(graph_path(root))?;

       if let Some(owner_repo) = github {
           // Fetch via GitHubSource
           use kgx::source::github::{GitHubSource, Layer};
           use kgx::Source;
           let layer: Layer = github_layer.parse()
               .map_err(|e: String| anyhow::anyhow!("{e}"))?;
           let source = GitHubSource::new(owner_repo, layer);
           let docs = source.fetch().map_err(|e| anyhow::anyhow!("{e}"))?;
           let mut total_nodes = 0;
           let mut total_edges = 0;
           for doc in &docs {
               let entities: Vec<_> = doc.entities.iter()
                   .map(|e| IngestEntity {
                       name: &e.name,
                       entity_type: &e.entity_type,
                       supporting_text: e.supporting_text.as_deref(),
                   })
                   .collect();
               let relations: Vec<_> = doc.relations.iter()
                   .map(|r| IngestRelation {
                       source: &r.source,
                       target: &r.target,
                       relation_type: &r.relation_type,
                       confidence: r.confidence,
                       supporting_text: r.supporting_text.as_deref(),
                   })
                   .collect();
               total_nodes += kgx::ingest_entities(&mut graph, &doc.doc_id, &entities);
               total_edges += kgx::ingest_relations(&mut graph, &doc.doc_id, &relations)
                   .map_err(|e| anyhow::anyhow!("{e}"))?;
           }
           graph.save()?;
           let out = IngestOutput {
               doc_id: format!("github:{owner_repo}"),
               chunk_count: 0,
               nodes_added: total_nodes,
               edges_added: total_edges,
           };
           println!("{}", serde_json::to_string_pretty(&out)?);
           return Ok(());
       }

       match format {
           "github" => {
               use kgx::parse::github::GitHubParser;
               use kgx::parse::Parser;
               let input_str = read_stdin()?;
               let doc = GitHubParser.parse(&input_str, "github:stdin")
                   .map_err(|e| anyhow::anyhow!("{e}"))?;
               let entities: Vec<_> = doc.entities.iter()
                   .map(|e| IngestEntity {
                       name: &e.name,
                       entity_type: &e.entity_type,
                       supporting_text: e.supporting_text.as_deref(),
                   })
                   .collect();
               let relations: Vec<_> = doc.relations.iter()
                   .map(|r| IngestRelation {
                       source: &r.source,
                       target: &r.target,
                       relation_type: &r.relation_type,
                       confidence: r.confidence,
                       supporting_text: r.supporting_text.as_deref(),
                   })
                   .collect();
               let nodes_added = kgx::ingest_entities(&mut graph, &doc.doc_id, &entities);
               let edges_added = kgx::ingest_relations(&mut graph, &doc.doc_id, &relations)
                   .map_err(|e| anyhow::anyhow!("{e}"))?;
               graph.save()?;
               let out = IngestOutput {
                   doc_id: doc.doc_id,
                   chunk_count: 0,
                   nodes_added,
                   edges_added,
               };
               println!("{}", serde_json::to_string_pretty(&out)?);
           }
           "json" | _ => {
               // Existing JSON ingest path
               let mut docs_store = DocumentStore::open(docs_path(root))?;
               let input: IngestInput = serde_json::from_str(&read_stdin()?)
                   .context("parsing ingest JSON from stdin")?;
               let chunk_count = docs_store
                   .ingest(&input.doc_id, &input.title, &input.source, &input.raw_content)
                   .chunks.len();
               let entities: Vec<_> = input.entities.iter()
                   .map(|e| IngestEntity {
                       name: &e.name,
                       entity_type: &e.entity_type,
                       supporting_text: e.supporting_text.as_deref(),
                   })
                   .collect();
               let relations: Vec<_> = input.relations.iter()
                   .map(|r| IngestRelation {
                       source: &r.source,
                       target: &r.target,
                       relation_type: &r.relation_type,
                       confidence: r.confidence,
                       supporting_text: r.supporting_text.as_deref(),
                   })
                   .collect();
               let nodes_added = kgx::ingest_entities(&mut graph, &input.doc_id, &entities);
               let edges_added = kgx::ingest_relations(&mut graph, &input.doc_id, &relations)
                   .map_err(|e| anyhow::anyhow!("{e}"))?;
               graph.save()?;
               docs_store.save()?;
               let out = IngestOutput {
                   doc_id: input.doc_id,
                   chunk_count,
                   nodes_added,
                   edges_added,
               };
               println!("{}", serde_json::to_string_pretty(&out)?);
           }
       }
       Ok(())
   }
   ```

4. Update the match arm in `main()`:

   ```rust
   Cmd::Ingest { format, github, github_layer } => {
       cmd_ingest(root, &format, github.as_deref(), &github_layer)
   }
   ```

5. Verify:

   ```
   cargo nextest run -p kgx-cli  -> all green
   cargo clippy -p kgx-cli -- -D warnings  -> zero warnings
   ```

6. Commit: `feat(kgx-cli): add --format github and --github flags to ingest`

### Task 7: Re-export and final integration test

**Crate**: `kgx`, `kgx-cli`
**File(s)**: `crates/kgx/src/lib.rs`, `crates/kgx-cli/tests/cli.rs`
**Run**: `cargo nextest run`

1. Ensure lib.rs re-exports:

   ```rust
   pub use parse::github::GitHubParser;
   pub use source::github::{GitHubSource, Layer};
   ```

2. Write integration test:

   ```rust
   // crates/kgx-cli/tests/cli.rs (append)
   #[test]
   fn ingest_github_format_creates_entities_queryable() {
       let dir = tempdir().unwrap();
       init_workspace(dir.path());

       let json = r#"{
           "full_name": "acme/widget",
           "description": "Widget factory",
           "owner": { "login": "acme", "type": "Organization" },
           "topics": ["widgets"],
           "language": "Go",
           "license": { "spdx_id": "Apache-2.0" }
       }"#;

       // Ingest
       Command::cargo_bin("kgx").unwrap()
           .args(["--root", dir.path().to_str().unwrap(), "ingest", "--format", "github"])
           .write_stdin(json)
           .assert()
           .success();

       // Query the repo entity
       Command::cargo_bin("kgx").unwrap()
           .args(["--root", dir.path().to_str().unwrap(), "query", "acme/widget"])
           .assert()
           .success()
           .stdout(predicates::str::contains("acme/widget"));
   }
   ```

3. Verify:

   ```
   cargo nextest run              -> all green
   cargo clippy -- -D warnings    -> zero warnings
   ```

4. Commit: `feat(kgx): complete GitHub ingestion with integration tests`
