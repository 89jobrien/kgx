# Plan: Parser Trait + Graph Operations

## Goal

Add a `Parser` trait and `GraphOp` trait to the `kgx` lib crate, with
Markdown parser and Merge operation as first implementations.

## Architecture

- Crates affected: `kgx` (lib only)
- New traits/types: `Parser`, `ParsedDocument`, `ParsedEntity`,
  `ParsedRelation`, `ParseError`, `MarkdownParser`, `GraphOp`, `MergeOp`,
  `MergeSummary`
- Data flow (parse): raw text -> `Parser::parse` -> `ParsedDocument` ->
  caller converts to `IngestEntity`/`IngestRelation` -> existing
  `ingest_entities`/`ingest_relations`
- Data flow (merge): two `GraphStore`s -> `MergeOp::apply` -> mutates
  target graph, returns `MergeSummary`

## Tech Stack

- Rust edition 2024
- Uses existing workspace dep `proptest` for property tests
- No other new dependencies -- markdown parsing via `str` methods

## Testing Strategy

| Dimension   | Where                         | What                                      |
| ----------- | ----------------------------- | ----------------------------------------- |
| Unit        | Each impl file `#[cfg(test)]` | Function-level behavior                   |
| Property    | `parse/markdown.rs`           | Invariants over generated markdown inputs |
| Conformance | `parse/mod.rs`, `ops/mod.rs`  | Reusable trait contract suites            |

## Tasks

### Task 1: Parser trait and ParsedDocument types

**Crate**: `kgx`
**File(s)**: `crates/kgx/src/parse/mod.rs`
**Run**: `cargo nextest run -p kgx -- parse`

1. Write failing test:

   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;

       struct EchoParser;

       impl Parser for EchoParser {
           fn parse(
               &self,
               _input: &str,
               doc_id: &str,
           ) -> Result<ParsedDocument, ParseError> {
               Ok(ParsedDocument {
                   doc_id: doc_id.to_string(),
                   entities: vec![ParsedEntity {
                       name: "test".to_string(),
                       entity_type: "echo".to_string(),
                       supporting_text: None,
                   }],
                   relations: vec![],
               })
           }
       }

       #[test]
       fn echo_parser_returns_parsed_document() {
           let p = EchoParser;
           let doc = p.parse("anything", "d1").unwrap();
           assert_eq!(doc.doc_id, "d1");
           assert_eq!(doc.entities.len(), 1);
           assert_eq!(doc.entities[0].name, "test");
           assert!(doc.relations.is_empty());
       }
   }
   ```

   Run: `cargo nextest run -p kgx -- echo_parser`
   Expected: FAIL (module doesn't exist)

2. Implement:

   ```rust
   pub mod markdown;

   /// A parsed entity, ready for ingestion.
   #[derive(Debug, Clone, PartialEq, Eq)]
   pub struct ParsedEntity {
       pub name: String,
       pub entity_type: String,
       pub supporting_text: Option<String>,
   }

   /// A parsed relation, ready for ingestion.
   #[derive(Debug, Clone, PartialEq)]
   pub struct ParsedRelation {
       pub source: String,
       pub target: String,
       pub relation_type: String,
       pub confidence: f64,
       pub supporting_text: Option<String>,
   }

   /// Output of a Parser.
   #[derive(Debug, Clone)]
   pub struct ParsedDocument {
       pub doc_id: String,
       pub entities: Vec<ParsedEntity>,
       pub relations: Vec<ParsedRelation>,
   }

   /// Errors from parsing.
   #[derive(Debug, thiserror::Error)]
   pub enum ParseError {
       #[error("parse failed: {0}")]
       Failed(String),
   }

   /// Converts a source format into entities and relations.
   pub trait Parser {
       fn parse(
           &self,
           input: &str,
           doc_id: &str,
       ) -> Result<ParsedDocument, ParseError>;
   }
   ```

3. Verify:

   ```
   cargo nextest run -p kgx -- parse   -> all green
   cargo clippy -p kgx -- -D warnings  -> zero warnings
   ```

4. Run: `git branch --show-current`
   Commit: `git commit -m "feat(kgx): add Parser trait and ParsedDocument types"`

### Task 2: Parser conformance suite

**Crate**: `kgx`
**File(s)**: `crates/kgx/src/parse/mod.rs` (add to existing `#[cfg(test)]`)
**Run**: `cargo nextest run -p kgx -- parser_conformance`

1. Write conformance tests in `parse/mod.rs`:

   ```rust
   /// Conformance test suite for the Parser trait.
   /// Any implementation must:
   /// 1. Return Ok for empty input (never panic)
   /// 2. Set doc_id on the returned ParsedDocument to match the input
   /// 3. Every relation source must appear as an entity name
   /// 4. Confidence values must be in [0.0, 1.0]
   #[cfg(test)]
   pub mod conformance {
       use super::*;

       pub fn assert_parser_contract(parser: &dyn Parser, label: &str) {
           // 1. Empty input succeeds
           let doc = parser
               .parse("", "empty-doc")
               .unwrap_or_else(|e| {
                   panic!("{label}: empty input must not error: {e}")
               });
           assert_eq!(
               doc.doc_id, "empty-doc",
               "{label}: doc_id must match input"
           );

           // 2. doc_id propagation with non-empty input
           let doc = parser
               .parse("# Hello\n\nSome content.\n", "test-doc")
               .unwrap_or_else(|e| {
                   panic!("{label}: simple input must not error: {e}")
               });
           assert_eq!(
               doc.doc_id, "test-doc",
               "{label}: doc_id must match input"
           );

           // 3. Every relation source must name an entity
           let entity_names: std::collections::HashSet<&str> = doc
               .entities
               .iter()
               .map(|e| e.name.as_str())
               .collect();
           for rel in &doc.relations {
               assert!(
                   entity_names.contains(rel.source.as_str()),
                   "{label}: relation source '{}' not found in entities",
                   rel.source
               );
           }

           // 4. Confidence in range
           for rel in &doc.relations {
               assert!(
                   (0.0..=1.0).contains(&rel.confidence),
                   "{label}: confidence {} out of [0.0, 1.0]",
                   rel.confidence
               );
           }
       }
   }
   ```

   Run: `cargo nextest run -p kgx -- parser_conformance`
   Expected: FAIL (no callers yet, but compiles; the test below will
   exercise it)

2. Add conformance test for `EchoParser` in the existing `tests` module:

   ```rust
   #[test]
   fn echo_parser_satisfies_contract() {
       conformance::assert_parser_contract(&EchoParser, "echo");
   }
   ```

3. Verify:

   ```
   cargo nextest run -p kgx -- parser_conformance  -> all green
   cargo clippy -p kgx -- -D warnings              -> zero warnings
   ```

4. Run: `git branch --show-current`
   Commit: `git commit -m "test(kgx): add Parser conformance suite"`

### Task 3: MarkdownParser

**Crate**: `kgx`
**File(s)**: `crates/kgx/src/parse/markdown.rs`
**Run**: `cargo nextest run -p kgx -- markdown`

1. Write failing tests:

   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;
       use crate::parse::conformance;

       #[test]
       fn markdown_parser_satisfies_contract() {
           conformance::assert_parser_contract(
               &MarkdownParser, "markdown",
           );
       }

       #[test]
       fn headings_become_entities() {
           let md = "# Top\n\nSome text.\n\n## Sub\n\nMore text.\n";
           let p = MarkdownParser;
           let doc = p.parse(md, "d1").unwrap();
           let names: Vec<&str> =
               doc.entities.iter().map(|e| e.name.as_str()).collect();
           assert!(names.contains(&"Top"));
           assert!(names.contains(&"Sub"));
           let top = doc
               .entities.iter().find(|e| e.name == "Top").unwrap();
           assert_eq!(top.entity_type, "section");
           let sub = doc
               .entities.iter().find(|e| e.name == "Sub").unwrap();
           assert_eq!(sub.entity_type, "heading");
       }

       #[test]
       fn wikilinks_become_relations() {
           let md = "## Source\n\nSee [[Target]] for details.\n";
           let p = MarkdownParser;
           let doc = p.parse(md, "d1").unwrap();
           assert_eq!(doc.relations.len(), 1);
           let r = &doc.relations[0];
           assert_eq!(r.source, "Source");
           assert_eq!(r.target, "Target");
           assert_eq!(r.relation_type, "references");
           assert_eq!(r.confidence, 1.0);
       }

       #[test]
       fn internal_links_become_relations() {
           let md = "## Page\n\nSee [other](other-page.md) here.\n";
           let p = MarkdownParser;
           let doc = p.parse(md, "d1").unwrap();
           assert_eq!(doc.relations.len(), 1);
           let r = &doc.relations[0];
           assert_eq!(r.source, "Page");
           assert_eq!(r.target, "other-page.md");
           assert_eq!(r.relation_type, "links_to");
           assert_eq!(r.confidence, 0.8);
       }

       #[test]
       fn external_links_ignored() {
           let md =
               "## Page\n\nSee [rust](https://rust-lang.org) here.\n";
           let p = MarkdownParser;
           let doc = p.parse(md, "d1").unwrap();
           assert!(doc.relations.is_empty());
       }

       #[test]
       fn no_headings_yields_empty() {
           let md = "Just some text without headings.\n";
           let p = MarkdownParser;
           let doc = p.parse(md, "d1").unwrap();
           assert!(doc.entities.is_empty());
           assert!(doc.relations.is_empty());
       }

       #[test]
       fn multiple_wikilinks_in_one_section() {
           let md = "## Hub\n\n[[A]] and [[B]] are linked.\n";
           let p = MarkdownParser;
           let doc = p.parse(md, "d1").unwrap();
           assert_eq!(doc.relations.len(), 2);
           assert!(doc.relations.iter().all(|r| r.source == "Hub"));
           let targets: Vec<&str> =
               doc.relations.iter().map(|r| r.target.as_str()).collect();
           assert!(targets.contains(&"A"));
           assert!(targets.contains(&"B"));
       }
   }
   ```

   Run: `cargo nextest run -p kgx -- markdown`
   Expected: FAIL

2. Implement:

   ```rust
   use super::{
       ParseError, ParsedDocument, ParsedEntity, ParsedRelation, Parser,
   };

   /// Parses markdown into entities (headings) and relations (links).
   pub struct MarkdownParser;

   impl Parser for MarkdownParser {
       fn parse(
           &self,
           input: &str,
           doc_id: &str,
       ) -> Result<ParsedDocument, ParseError> {
           let mut entities = Vec::new();
           let mut relations = Vec::new();
           let mut current_heading: Option<String> = None;

           for line in input.lines() {
               if let Some(title) = line.strip_prefix("## ") {
                   let title = title.trim().to_string();
                   entities.push(ParsedEntity {
                       name: title.clone(),
                       entity_type: "heading".to_string(),
                       supporting_text: None,
                   });
                   current_heading = Some(title);
               } else if let Some(title) = line.strip_prefix("# ") {
                   let title = title.trim().to_string();
                   entities.push(ParsedEntity {
                       name: title.clone(),
                       entity_type: "section".to_string(),
                       supporting_text: None,
                   });
                   current_heading = Some(title);
               } else if let Some(ref heading) = current_heading {
                   extract_wikilinks(
                       line, heading, &mut relations,
                   );
                   extract_md_links(
                       line, heading, &mut relations,
                   );
               }
           }

           Ok(ParsedDocument {
               doc_id: doc_id.to_string(),
               entities,
               relations,
           })
       }
   }

   fn extract_wikilinks(
       line: &str,
       heading: &str,
       relations: &mut Vec<ParsedRelation>,
   ) {
       let mut rest = line;
       while let Some(start) = rest.find("[[") {
           let after = &rest[start + 2..];
           if let Some(end) = after.find("]]") {
               let target = &after[..end];
               if !target.is_empty() {
                   relations.push(ParsedRelation {
                       source: heading.to_string(),
                       target: target.to_string(),
                       relation_type: "references".to_string(),
                       confidence: 1.0,
                       supporting_text: None,
                   });
               }
               rest = &after[end + 2..];
           } else {
               break;
           }
       }
   }

   fn extract_md_links(
       line: &str,
       heading: &str,
       relations: &mut Vec<ParsedRelation>,
   ) {
       let mut rest = line;
       while let Some(bracket_start) = rest.find('[') {
           let after_bracket = &rest[bracket_start + 1..];
           if let Some(bracket_end) = after_bracket.find("](") {
               let paren_content = &after_bracket[bracket_end + 2..];
               if let Some(paren_end) = paren_content.find(')') {
                   let url = &paren_content[..paren_end];
                   if !url.starts_with("http://")
                       && !url.starts_with("https://")
                       && !url.is_empty()
                   {
                       relations.push(ParsedRelation {
                           source: heading.to_string(),
                           target: url.to_string(),
                           relation_type: "links_to".to_string(),
                           confidence: 0.8,
                           supporting_text: None,
                       });
                   }
                   rest = &paren_content[paren_end + 1..];
               } else {
                   break;
               }
           } else {
               break;
           }
       }
   }
   ```

3. Verify:

   ```
   cargo nextest run -p kgx -- markdown  -> all green
   cargo clippy -p kgx -- -D warnings    -> zero warnings
   ```

4. Run: `git branch --show-current`
   Commit: `git commit -m "feat(kgx): add MarkdownParser implementation"`

### Task 4: MarkdownParser property tests

**Crate**: `kgx`
**File(s)**: `crates/kgx/src/parse/markdown.rs` (append to `#[cfg(test)]`)
**Run**: `cargo nextest run -p kgx -- markdown_prop`

1. Add property tests to the existing `tests` module in `markdown.rs`:

   ```rust
   mod prop {
       use super::*;
       use proptest::prelude::*;

       /// Generate a random markdown document with headings and
       /// wikilinks. The generator ensures structure is valid so
       /// we can assert invariants on the output.
       fn arb_markdown() -> impl Strategy<Value = String> {
           // 1-5 sections, each with a heading and 0-3 wikilinks
           prop::collection::vec(
               (
                   "[A-Za-z][A-Za-z0-9 ]{0,20}",
                   prop::collection::vec(
                       "[A-Za-z][A-Za-z0-9]{0,10}",
                       0..=3,
                   ),
               ),
               1..=5,
           )
           .prop_map(|sections| {
               let mut md = String::new();
               for (heading, links) in sections {
                   md.push_str(&format!("## {heading}\n\n"));
                   for link in links {
                       md.push_str(&format!("See [[{link}]].\n"));
                   }
                   md.push('\n');
               }
               md
           })
       }

       proptest! {
           #[test]
           fn markdown_prop_never_panics(input in ".*") {
               let p = MarkdownParser;
               let _ = p.parse(&input, "prop-doc");
           }

           #[test]
           fn markdown_prop_relation_sources_are_entities(
               md in arb_markdown()
           ) {
               let p = MarkdownParser;
               let doc = p.parse(&md, "prop-doc")
                   .expect("structured markdown must parse");
               let entity_names: std::collections::HashSet<&str> = doc
                   .entities
                   .iter()
                   .map(|e| e.name.as_str())
                   .collect();
               for rel in &doc.relations {
                   prop_assert!(
                       entity_names.contains(rel.source.as_str()),
                       "relation source '{}' not in entities",
                       rel.source
                   );
               }
           }

           #[test]
           fn markdown_prop_confidence_in_range(
               md in arb_markdown()
           ) {
               let p = MarkdownParser;
               let doc = p.parse(&md, "prop-doc")
                   .expect("structured markdown must parse");
               for rel in &doc.relations {
                   prop_assert!(
                       (0.0..=1.0).contains(&rel.confidence),
                       "confidence {} out of range",
                       rel.confidence
                   );
               }
           }

           #[test]
           fn markdown_prop_doc_id_preserved(
               md in arb_markdown(),
               doc_id in "[a-z]{1,10}"
           ) {
               let p = MarkdownParser;
               let doc = p.parse(&md, &doc_id)
                   .expect("structured markdown must parse");
               prop_assert_eq!(doc.doc_id, doc_id);
           }
       }
   }
   ```

2. Verify:

   ```
   cargo nextest run -p kgx -- markdown_prop  -> all green
   cargo clippy -p kgx -- -D warnings         -> zero warnings
   ```

3. Run: `git branch --show-current`
   Commit: `git commit -m "test(kgx): add MarkdownParser property tests"`

### Task 5: GraphOp trait

**Crate**: `kgx`
**File(s)**: `crates/kgx/src/ops/mod.rs`
**Run**: `cargo nextest run -p kgx -- graph_op`

1. Write failing test:

   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;
       use crate::GraphStore;

       struct CountNodes;

       impl GraphOp for CountNodes {
           type Output = usize;

           fn apply(
               self,
               graph: &mut GraphStore,
           ) -> anyhow::Result<usize> {
               Ok(graph.node_count())
           }
       }

       #[test]
       fn graph_op_trait_works() {
           let mut g = GraphStore::open(format!(
               "/tmp/kgx_test_op_{}.json",
               uuid::Uuid::new_v4()
           ))
           .unwrap();
           g.add_node("A", "t", None, None);
           let count = CountNodes.apply(&mut g).unwrap();
           assert_eq!(count, 1);
       }
   }
   ```

   Run: `cargo nextest run -p kgx -- graph_op`
   Expected: FAIL

2. Implement:

   ```rust
   pub mod merge;

   use crate::GraphStore;

   /// A graph operation (command pattern).
   pub trait GraphOp {
       type Output;
       fn apply(self, graph: &mut GraphStore) -> anyhow::Result<Self::Output>;
   }
   ```

3. Verify:

   ```
   cargo nextest run -p kgx -- graph_op  -> all green
   cargo clippy -p kgx -- -D warnings    -> zero warnings
   ```

4. Run: `git branch --show-current`
   Commit: `git commit -m "feat(kgx): add GraphOp trait"`

### Task 6: GraphOp conformance suite

**Crate**: `kgx`
**File(s)**: `crates/kgx/src/ops/mod.rs` (add to existing `#[cfg(test)]`)
**Run**: `cargo nextest run -p kgx -- graph_op_conformance`

1. Write conformance test infrastructure:

   ```rust
   /// Conformance test suite for the GraphOp trait.
   /// Any implementation must:
   /// 1. Not reduce node_count (operations should be additive or neutral)
   /// 2. Return Ok (not panic) on an empty graph
   /// 3. Return Ok (not panic) on a populated graph
   #[cfg(test)]
   pub mod conformance {
       use super::*;
       use crate::{EdgeInput, GraphStore};

       pub fn empty_graph() -> GraphStore {
           GraphStore::open(format!(
               "/tmp/kgx_test_conformance_{}.json",
               uuid::Uuid::new_v4()
           ))
           .expect("fresh graph should open")
       }

       pub fn populated_graph() -> GraphStore {
           let mut g = empty_graph();
           let a = g.add_node("Alpha", "concept", Some("first"), Some("d1"));
           let b = g.add_node("Beta", "concept", Some("second"), Some("d1"));
           g.add_edge(EdgeInput {
               source: a,
               target: b,
               relation_type: "precedes",
               confidence: 0.8,
               supporting_text: None,
               source_doc: Some("d1"),
           });
           g
       }

       /// Run the conformance suite. `make_op_empty` and `make_op_pop`
       /// are closures that produce a fresh op instance for each test
       /// scenario (since GraphOp consumes self).
       pub fn assert_graph_op_contract<T: GraphOp>(
           make_op_empty: impl FnOnce() -> T,
           make_op_pop: impl FnOnce() -> T,
           label: &str,
       ) {
           // 1. Succeeds on empty graph
           {
               let mut g = empty_graph();
               let op = make_op_empty();
               op.apply(&mut g).unwrap_or_else(|e| {
                   panic!("{label}: must not error on empty graph: {e}")
               });
           }

           // 2. Succeeds on populated graph without reducing node count
           {
               let mut g = populated_graph();
               let before = g.node_count();
               let op = make_op_pop();
               op.apply(&mut g).unwrap_or_else(|e| {
                   panic!("{label}: must not error on populated graph: {e}")
               });
               assert!(
                   g.node_count() >= before,
                   "{label}: node_count must not decrease (was {before}, now {})",
                   g.node_count()
               );
           }
       }
   }
   ```

2. Add conformance test for `CountNodes` in the existing `tests` module:

   ```rust
   #[test]
   fn count_nodes_satisfies_contract() {
       conformance::assert_graph_op_contract(
           || CountNodes,
           || CountNodes,
           "count_nodes",
       );
   }
   ```

3. Verify:

   ```
   cargo nextest run -p kgx -- graph_op_conformance  -> all green
   cargo clippy -p kgx -- -D warnings                -> zero warnings
   ```

4. Run: `git branch --show-current`
   Commit: `git commit -m "test(kgx): add GraphOp conformance suite"`

### Task 7: MergeOp

**Crate**: `kgx`
**File(s)**: `crates/kgx/src/ops/merge.rs`
**Run**: `cargo nextest run -p kgx -- merge`

1. Write failing tests:

   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;
       use crate::ops::conformance;
       use crate::{EdgeInput, GraphStore};

       fn fresh_graph() -> GraphStore {
           GraphStore::open(format!(
               "/tmp/kgx_test_merge_{}.json",
               uuid::Uuid::new_v4()
           ))
           .unwrap()
       }

       #[test]
       fn merge_op_satisfies_contract() {
           conformance::assert_graph_op_contract(
               || MergeOp { source: fresh_graph() },
               || {
                   let mut s = fresh_graph();
                   s.add_node("Gamma", "concept", None, None);
                   MergeOp { source: s }
               },
               "merge",
           );
       }

       #[test]
       fn merge_adds_new_nodes() {
           let mut target = fresh_graph();
           target.add_node("A", "t", None, None);

           let mut source = fresh_graph();
           source.add_node("B", "t", None, None);

           let op = MergeOp { source };
           let summary = op.apply(&mut target).unwrap();
           assert_eq!(summary.nodes_added, 1);
           assert_eq!(summary.nodes_merged, 0);
           assert_eq!(target.node_count(), 2);
       }

       #[test]
       fn merge_deduplicates_by_name() {
           let mut target = fresh_graph();
           target.add_node("A", "t", None, Some("d1"));

           let mut source = fresh_graph();
           source.add_node("a", "t", None, Some("d2"));

           let op = MergeOp { source };
           let summary = op.apply(&mut target).unwrap();
           assert_eq!(summary.nodes_added, 0);
           assert_eq!(summary.nodes_merged, 1);
           assert_eq!(target.node_count(), 1);
           let id = target.node_by_name("a").unwrap();
           let node = target.get_node(id).unwrap();
           assert!(node.source_docs.contains(&"d1".to_string()));
           assert!(node.source_docs.contains(&"d2".to_string()));
       }

       #[test]
       fn merge_adds_edges() {
           let mut target = fresh_graph();
           let a = target.add_node("A", "t", None, None);
           let b = target.add_node("B", "t", None, None);
           target.add_edge(EdgeInput {
               source: a,
               target: b,
               relation_type: "r1",
               confidence: 1.0,
               supporting_text: None,
               source_doc: None,
           });

           let mut source = fresh_graph();
           let sa = source.add_node("A", "t", None, None);
           let sc = source.add_node("C", "t", None, None);
           source.add_edge(EdgeInput {
               source: sa,
               target: sc,
               relation_type: "r2",
               confidence: 0.9,
               supporting_text: None,
               source_doc: None,
           });

           let op = MergeOp { source };
           let summary = op.apply(&mut target).unwrap();
           assert_eq!(summary.edges_added, 1);
           assert_eq!(target.edge_count(), 2);
           assert_eq!(target.node_count(), 3);
       }

       #[test]
       fn merge_skips_duplicate_edges() {
           let mut target = fresh_graph();
           let a = target.add_node("A", "t", None, None);
           let b = target.add_node("B", "t", None, None);
           target.add_edge(EdgeInput {
               source: a,
               target: b,
               relation_type: "rel",
               confidence: 1.0,
               supporting_text: None,
               source_doc: None,
           });

           let mut source = fresh_graph();
           let sa = source.add_node("A", "t", None, None);
           let sb = source.add_node("B", "t", None, None);
           source.add_edge(EdgeInput {
               source: sa,
               target: sb,
               relation_type: "rel",
               confidence: 0.8,
               supporting_text: None,
               source_doc: None,
           });

           let op = MergeOp { source };
           let summary = op.apply(&mut target).unwrap();
           assert_eq!(summary.edges_added, 0);
           assert_eq!(target.edge_count(), 1);
       }

       #[test]
       fn merge_low_confidence_edge_skipped() {
           let mut target = fresh_graph();
           target.add_node("A", "t", None, None);

           let mut source = fresh_graph();
           let sa = source.add_node("A", "t", None, None);
           let sb = source.add_node("B", "t", None, None);
           source.add_edge(EdgeInput {
               source: sa,
               target: sb,
               relation_type: "weak",
               confidence: 0.3,
               supporting_text: None,
               source_doc: None,
           });

           let op = MergeOp { source };
           let summary = op.apply(&mut target).unwrap();
           assert_eq!(summary.edges_added, 0);
       }
   }
   ```

   Run: `cargo nextest run -p kgx -- merge`
   Expected: FAIL

2. Implement:

   ```rust
   use super::GraphOp;
   use crate::{EdgeInput, GraphStore};

   /// Merge another graph into the target.
   pub struct MergeOp {
       pub source: GraphStore,
   }

   /// Result of a merge operation.
   #[derive(Debug, Clone, PartialEq, Eq)]
   pub struct MergeSummary {
       pub nodes_added: usize,
       pub nodes_merged: usize,
       pub edges_added: usize,
   }

   impl GraphOp for MergeOp {
       type Output = MergeSummary;

       fn apply(
           self,
           graph: &mut GraphStore,
       ) -> anyhow::Result<MergeSummary> {
           let mut nodes_added: usize = 0;
           let mut nodes_merged: usize = 0;
           let mut edges_added: usize = 0;

           let source_nodes: Vec<_> =
               self.source.nodes().cloned().collect();
           for node in &source_nodes {
               let existed = graph.node_by_name(&node.name).is_some();
               graph.add_node(
                   &node.name,
                   &node.entity_type,
                   node.supporting_text.as_deref(),
                   None,
               );
               // Merge source_docs from the source node
               for doc in &node.source_docs {
                   graph.add_node(
                       &node.name,
                       &node.entity_type,
                       node.supporting_text.as_deref(),
                       Some(doc),
                   );
               }
               if existed {
                   nodes_merged += 1;
               } else {
                   nodes_added += 1;
               }
           }

           // Build edge dedup set: (source_name, target_name, relation_type)
           let existing_edges: std::collections::HashSet<
               (String, String, String),
           > = graph
               .edges()
               .iter()
               .filter_map(|e| {
                   let src = graph.get_node(e.source)?;
                   let tgt = graph.get_node(e.target)?;
                   Some((
                       src.name.to_lowercase(),
                       tgt.name.to_lowercase(),
                       e.relation_type.clone(),
                   ))
               })
               .collect();

           for edge in self.source.edges() {
               let src_name = self
                   .source
                   .get_node(edge.source)
                   .map(|n| n.name.clone())
                   .unwrap_or_default();
               let tgt_name = self
                   .source
                   .get_node(edge.target)
                   .map(|n| n.name.clone())
                   .unwrap_or_default();
               let key = (
                   src_name.to_lowercase(),
                   tgt_name.to_lowercase(),
                   edge.relation_type.clone(),
               );
               if existing_edges.contains(&key) {
                   continue;
               }
               let Some(src_id) = graph.node_by_name(&src_name) else {
                   continue;
               };
               let Some(tgt_id) = graph.node_by_name(&tgt_name) else {
                   continue;
               };
               if graph
                   .add_edge(EdgeInput {
                       source: src_id,
                       target: tgt_id,
                       relation_type: &edge.relation_type,
                       confidence: edge.confidence,
                       supporting_text: edge.supporting_text.as_deref(),
                       source_doc: edge.source_doc.as_deref(),
                   })
                   .is_some()
               {
                   edges_added += 1;
               }
           }

           Ok(MergeSummary {
               nodes_added,
               nodes_merged,
               edges_added,
           })
       }
   }
   ```

3. Verify:

   ```
   cargo nextest run -p kgx -- merge    -> all green
   cargo clippy -p kgx -- -D warnings   -> zero warnings
   ```

4. Run: `git branch --show-current`
   Commit: `git commit -m "feat(kgx): add MergeOp implementation"`

### Task 8: Wire modules into lib.rs

**Crate**: `kgx`
**File(s)**: `crates/kgx/src/lib.rs`
**Run**: `cargo nextest run -p kgx`

1. Add to `lib.rs`:

   ```rust
   pub mod ops;
   pub mod parse;
   ```

   Add re-exports:

   ```rust
   pub use ops::{GraphOp, MergeOp, MergeSummary};
   pub use parse::{
       MarkdownParser, ParseError, ParsedDocument, ParsedEntity,
       ParsedRelation, Parser,
   };
   ```

2. Verify:

   ```
   cargo nextest run -p kgx             -> all green
   cargo clippy -p kgx -- -D warnings   -> zero warnings
   ```

3. Run: `git branch --show-current`
   Commit: `git commit -m "feat(kgx): re-export parse and ops modules"`
