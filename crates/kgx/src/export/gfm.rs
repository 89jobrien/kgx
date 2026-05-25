use std::collections::HashMap;
use std::fs;
use std::path::Path;

use super::{ExportContext, Exporter};
use crate::types::{Entity, NodeId};
use crate::wiki::slugify;

/// Escape pipe characters for GFM table cells.
fn escape_pipe(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', " ")
}

/// Exports the full context graph as a single GitHub-flavored markdown file.
pub struct GfmExporter;

impl Exporter for GfmExporter {
    fn export(&self, ctx: &ExportContext, output: &Path) -> anyhow::Result<()> {
        fs::create_dir_all(output)?;

        let mut entities: Vec<Entity> = ctx.graph.nodes().cloned().collect();
        entities.sort_by(|a, b| a.name.cmp(&b.name));
        let edges = ctx.graph.edges();
        let node_map: HashMap<NodeId, &Entity> = entities.iter().map(|e| (e.id, e)).collect();
        let docs: Vec<_> = ctx.docs.list().collect();
        let wiki_pages = ctx.wiki.pages()?;
        let doc_count = docs.len();

        let mut md = String::new();

        // Header and stats
        md.push_str("# kgx Export\n\n");
        md.push_str("| Metric | Count |\n");
        md.push_str("|--------|-------|\n");
        md.push_str(&format!("| Entities | {} |\n", entities.len()));
        md.push_str(&format!("| Relations | {} |\n", edges.len()));
        md.push_str(&format!("| Documents | {doc_count} |\n"));
        md.push_str(&format!("| Wiki Pages | {} |\n", wiki_pages.len()));
        md.push('\n');

        // Entities table
        if !entities.is_empty() {
            md.push_str("## Entities\n\n");
            md.push_str("| Name | Type | Supporting Text | Source Docs |\n");
            md.push_str("|------|------|-----------------|-------------|\n");
            for entity in &entities {
                let name = escape_pipe(&entity.name);
                let etype = escape_pipe(&entity.entity_type);
                let text = entity
                    .supporting_text
                    .as_deref()
                    .map(escape_pipe)
                    .unwrap_or_default();
                let docs_col = escape_pipe(&entity.source_docs.join(", "));
                md.push_str(&format!("| {name} | {etype} | {text} | {docs_col} |\n"));
            }
            md.push('\n');
        }

        // Relations table
        if !edges.is_empty() {
            md.push_str("## Relations\n\n");
            md.push_str("| Source | Relation | Target | Confidence |\n");
            md.push_str("|--------|----------|--------|------------|\n");
            for rel in edges {
                let src_name = node_map
                    .get(&rel.source)
                    .map(|e| escape_pipe(&e.name))
                    .unwrap_or_else(|| "?".to_string());
                let tgt_name = node_map
                    .get(&rel.target)
                    .map(|e| escape_pipe(&e.name))
                    .unwrap_or_else(|| "?".to_string());
                let rtype = escape_pipe(&rel.relation_type);
                md.push_str(&format!(
                    "| {src_name} | {rtype} | {tgt_name} | {:.2} |\n",
                    rel.confidence,
                ));
            }
            md.push('\n');
        }

        // Documents
        if !docs.is_empty() {
            md.push_str("## Documents\n\n");
            for doc in &docs {
                md.push_str(&format!("### {}\n\n", doc.title));
                md.push_str(&format!("- **ID:** `{}`\n", doc.id));
                md.push_str(&format!("- **Source:** `{}`\n", doc.source));
                md.push_str(&format!("- **Chunks:** {}\n\n", doc.chunks.len()));
                if !doc.chunks.is_empty() {
                    md.push_str("<details>\n<summary>Chunks</summary>\n\n");
                    for (i, chunk) in doc.chunks.iter().enumerate() {
                        md.push_str(&format!("**Chunk {i}** (offset {}):\n\n", chunk.offset));
                        md.push_str(&format!("```\n{}\n```\n\n", chunk.text));
                    }
                    md.push_str("</details>\n\n");
                }
            }
        }

        // Wiki pages
        if !wiki_pages.is_empty() {
            md.push_str("## Wiki Pages\n\n");
            md.push_str("| Page | Category | Summary |\n");
            md.push_str("|------|----------|---------|\n");
            for page in &wiki_pages {
                let slug = slugify(&page.title);
                let summary = escape_pipe(&page.summary);
                md.push_str(&format!(
                    "| [{}](#{}) | {} | {} |\n",
                    page.title, slug, page.category, summary,
                ));
            }
            md.push('\n');

            for page in &wiki_pages {
                let slug = slugify(&page.title);
                md.push_str(&format!("### <a id=\"{slug}\"></a>{}\n\n", page.title));
                md.push_str(&format!(
                    "*Category: {} | Slug: `{}`*\n\n",
                    page.category, page.slug,
                ));
                md.push_str(&page.content);
                md.push_str("\n\n---\n\n");
            }
        }

        fs::write(output.join("kgx-export.md"), md)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::WikiCategory;
    use crate::{DocumentStore, EdgeInput, GraphStore, WikiStore};
    use proptest::prelude::*;
    use uuid::Uuid;

    fn test_dir() -> String {
        format!("/tmp/kgx_test_gfm_export_{}", Uuid::new_v4())
    }

    fn setup_stores(dir: &str) -> (GraphStore, DocumentStore, WikiStore) {
        let mut graph = GraphStore::open(format!("{dir}/graph.json")).expect("graph");
        let a = graph.add_node("Rust", "language", Some("Systems language"), Some("d1"));
        let b = graph.add_node("Cargo", "tool", Some("Rust package manager"), Some("d1"));
        graph.add_edge(EdgeInput {
            source: a,
            target: b,
            relation_type: "uses",
            confidence: 0.95,
            supporting_text: Some("Rust uses Cargo"),
            source_doc: Some("d1"),
        });

        let mut docs = DocumentStore::open(format!("{dir}/docs.json")).expect("docs");
        docs.ingest("d1", "Intro to Rust", "src.md", "Rust is great.");

        let wiki = WikiStore::open(format!("{dir}/wiki")).expect("wiki");
        wiki.write_page(
            WikiCategory::Entity,
            "Rust",
            "Rust wiki content",
            "Rust summary",
        )
        .expect("write");

        (graph, docs, wiki)
    }

    #[test]
    fn gfm_export_writes_single_file() {
        let dir = test_dir();
        let (graph, docs, wiki) = setup_stores(&dir);
        let ctx = ExportContext {
            graph: &graph,
            docs: &docs,
            wiki: &wiki,
        };
        let out = format!("{dir}/output");
        GfmExporter.export(&ctx, Path::new(&out)).expect("export");

        assert!(Path::new(&format!("{out}/kgx-export.md")).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn gfm_export_contains_tables() {
        let dir = test_dir();
        let (graph, docs, wiki) = setup_stores(&dir);
        let ctx = ExportContext {
            graph: &graph,
            docs: &docs,
            wiki: &wiki,
        };
        let out = format!("{dir}/output");
        GfmExporter.export(&ctx, Path::new(&out)).expect("export");

        let content = std::fs::read_to_string(format!("{out}/kgx-export.md")).expect("read");
        assert!(content.contains("| Entities | 2 |"));
        assert!(content.contains("| Relations | 1 |"));
        assert!(content.contains("| Rust | language |"));
        assert!(content.contains("| Cargo | tool |"));
        assert!(content.contains("| Rust | uses | Cargo | 0.95 |"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn gfm_export_contains_documents_and_wiki() {
        let dir = test_dir();
        let (graph, docs, wiki) = setup_stores(&dir);
        let ctx = ExportContext {
            graph: &graph,
            docs: &docs,
            wiki: &wiki,
        };
        let out = format!("{dir}/output");
        GfmExporter.export(&ctx, Path::new(&out)).expect("export");

        let content = std::fs::read_to_string(format!("{out}/kgx-export.md")).expect("read");
        assert!(content.contains("### Intro to Rust"));
        assert!(content.contains("<details>"));
        assert!(content.contains("## Wiki Pages"));
        assert!(content.contains("Rust wiki content"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn gfm_export_escapes_pipes_in_entity_names() {
        let dir = test_dir();
        let mut graph = GraphStore::open(format!("{dir}/graph.json")).expect("graph");
        graph.add_node("A | B", "type|x", Some("text | here"), None);
        let docs = DocumentStore::open(format!("{dir}/docs.json")).expect("docs");
        let wiki = WikiStore::open(format!("{dir}/wiki")).expect("wiki");

        let ctx = ExportContext {
            graph: &graph,
            docs: &docs,
            wiki: &wiki,
        };
        let out = format!("{dir}/output");
        GfmExporter.export(&ctx, Path::new(&out)).expect("export");

        let content = std::fs::read_to_string(format!("{out}/kgx-export.md")).expect("read");
        assert!(
            content.contains("A \\| B"),
            "pipe in entity name should be escaped"
        );
        assert!(
            content.contains("type\\|x"),
            "pipe in entity type should be escaped"
        );
        assert!(
            content.contains("text \\| here"),
            "pipe in supporting text should be escaped"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    // Property: every entity row has exactly 5 pipe characters (the 4 column
    // separators plus the trailing one), regardless of entity name content.
    proptest! {
        #[test]
        fn entity_table_rows_have_correct_pipe_count(
            name in "[a-zA-Z0-9|\\- ]{1,40}",
            etype in "[a-z]{1,10}",
            text in "[a-zA-Z0-9|\\-\\n ]{0,60}",
        ) {
            let dir = test_dir();
            let mut graph = GraphStore::open(format!("{dir}/graph.json")).expect("graph");
            graph.add_node(&name, &etype, Some(&text), None);
            let docs = DocumentStore::open(format!("{dir}/docs.json")).expect("docs");
            let wiki = WikiStore::open(format!("{dir}/wiki")).expect("wiki");

            let ctx = ExportContext { graph: &graph, docs: &docs, wiki: &wiki };
            let out = format!("{dir}/output");
            GfmExporter.export(&ctx, Path::new(&out)).expect("export");

            let content = std::fs::read_to_string(format!("{out}/kgx-export.md")).expect("read");
            // Find entity data rows (skip header and separator)
            let entity_lines: Vec<&str> = content
                .lines()
                .skip_while(|l| !l.starts_with("|------|"))
                .skip(1) // separator
                .take_while(|l| l.starts_with('|'))
                .collect();

            for line in &entity_lines {
                // Unescaped pipes: count '|' that are NOT preceded by '\'
                let unescaped = line.chars()
                    .enumerate()
                    .filter(|&(i, c)| c == '|' && (i == 0 || line.as_bytes()[i - 1] != b'\\'))
                    .count();
                prop_assert_eq!(unescaped, 5, "row should have 5 unescaped pipes: {}", line);
            }

            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    #[test]
    fn gfm_export_empty_graph() {
        let dir = test_dir();
        let graph = GraphStore::open(format!("{dir}/graph.json")).expect("graph");
        let docs = DocumentStore::open(format!("{dir}/docs.json")).expect("docs");
        let wiki = WikiStore::open(format!("{dir}/wiki")).expect("wiki");

        let ctx = ExportContext {
            graph: &graph,
            docs: &docs,
            wiki: &wiki,
        };
        let out = format!("{dir}/output");
        GfmExporter.export(&ctx, Path::new(&out)).expect("export");

        let content = std::fs::read_to_string(format!("{out}/kgx-export.md")).expect("read");
        assert!(content.contains("| Entities | 0 |"));
        assert!(!content.contains("## Entities"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
