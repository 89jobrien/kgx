use std::collections::HashMap;
use std::fs;
use std::path::Path;

use super::{ExportContext, Exporter};
use crate::types::{Entity, NodeId, Relation, WikiPage};
use crate::wiki::slugify;

pub struct MarkdownExporter;

impl Exporter for MarkdownExporter {
    fn export(&self, ctx: &ExportContext, output: &Path) -> anyhow::Result<()> {
        fs::create_dir_all(output.join("entities"))?;
        fs::create_dir_all(output.join("documents"))?;

        let entities: Vec<Entity> = ctx.graph.nodes().cloned().collect();
        let edges = ctx.graph.edges();
        let node_map: HashMap<NodeId, &Entity> = entities.iter().map(|e| (e.id, e)).collect();

        let wiki_pages = ctx.wiki.pages()?;

        for entity in &entities {
            write_entity_file(output, entity, edges, &node_map, ctx, &wiki_pages)?;
        }

        for doc in ctx.docs.list() {
            write_document_file(output, doc)?;
        }

        for page in &wiki_pages {
            let dir = output.join("wiki").join(page.category.as_dir());
            fs::create_dir_all(&dir)?;
            write_wiki_file(&dir, page, &entities)?;
        }

        write_index_file(output, &entities, edges, ctx, &wiki_pages)?;

        Ok(())
    }
}

fn write_entity_file(
    output: &Path,
    entity: &Entity,
    edges: &[Relation],
    node_map: &HashMap<NodeId, &Entity>,
    ctx: &ExportContext,
    wiki_pages: &[WikiPage],
) -> anyhow::Result<()> {
    let slug = slugify(&entity.name);
    let mut md = String::new();

    // Frontmatter
    md.push_str("---\n");
    md.push_str(&format!("id: \"{}\"\n", entity.id));
    md.push_str(&format!("type: {}\n", entity.entity_type));
    if !entity.source_docs.is_empty() {
        md.push_str("source_docs:\n");
        for doc_id in &entity.source_docs {
            md.push_str(&format!("  - \"{doc_id}\"\n"));
        }
    }
    md.push_str("---\n\n");

    // Title
    md.push_str(&format!("# {}\n\n", entity.name));

    // Supporting text
    if let Some(text) = &entity.supporting_text {
        md.push_str(&format!("{text}\n\n"));
    }

    // Relations
    let outgoing: Vec<&Relation> = edges.iter().filter(|e| e.source == entity.id).collect();
    let incoming: Vec<&Relation> = edges.iter().filter(|e| e.target == entity.id).collect();

    if !outgoing.is_empty() || !incoming.is_empty() {
        md.push_str("## Relations\n\n");
        for rel in &outgoing {
            if let Some(target) = node_map.get(&rel.target) {
                let target_slug = slugify(&target.name);
                md.push_str(&format!(
                    "- **{}** -> [[entities/{}|{}]] (confidence: {:.2})\n",
                    rel.relation_type, target_slug, target.name, rel.confidence,
                ));
            }
        }
        for rel in &incoming {
            if let Some(source) = node_map.get(&rel.source) {
                let source_slug = slugify(&source.name);
                md.push_str(&format!(
                    "- **{}** <- [[entities/{}|{}]] (confidence: {:.2})\n",
                    rel.relation_type, source_slug, source.name, rel.confidence,
                ));
            }
        }
        md.push('\n');
    }

    // Source documents
    if !entity.source_docs.is_empty() {
        md.push_str("## Source Documents\n\n");
        for doc_id in &entity.source_docs {
            let title = ctx
                .docs
                .get(doc_id)
                .map(|d| d.title.as_str())
                .unwrap_or(doc_id);
            md.push_str(&format!("- [[documents/{doc_id}|{title}]]\n"));
        }
        md.push('\n');
    }

    // Relevant chunks
    let chunks: Vec<_> = entity
        .source_docs
        .iter()
        .filter_map(|id| ctx.docs.get(id))
        .flat_map(|d| d.chunks.iter().enumerate().map(move |pair| (d, pair)))
        .collect();

    if !chunks.is_empty() {
        md.push_str("## Relevant Chunks\n\n");
        for (doc, (i, chunk)) in &chunks {
            let text = chunk.text.replace('\n', "\n> ");
            md.push_str(&format!(
                "> {text}\n> -- from [[documents/{}|{}]], chunk {i}\n\n",
                doc.id, doc.title,
            ));
        }
    }

    // Wiki pages (match by slug)
    let matching: Vec<_> = wiki_pages.iter().filter(|p| p.slug == slug).collect();
    if !matching.is_empty() {
        md.push_str("## Wiki Pages\n\n");
        for page in &matching {
            md.push_str(&format!(
                "- [[wiki/{}/{}|{} (wiki)]]\n",
                page.category.as_dir(),
                page.slug,
                entity.name,
            ));
        }
        md.push('\n');
    }

    fs::write(output.join("entities").join(format!("{slug}.md")), md)?;
    Ok(())
}

fn write_document_file(output: &Path, doc: &crate::types::Document) -> anyhow::Result<()> {
    let mut md = String::new();

    md.push_str("---\n");
    md.push_str(&format!("id: \"{}\"\n", doc.id));
    md.push_str(&format!("title: \"{}\"\n", doc.title));
    md.push_str(&format!("source: \"{}\"\n", doc.source));
    md.push_str(&format!("chunk_count: {}\n", doc.chunks.len()));
    md.push_str("---\n\n");

    md.push_str(&format!("# {}\n\n", doc.title));
    md.push_str(&format!("{}\n\n", doc.raw_content));

    if !doc.chunks.is_empty() {
        md.push_str("## Chunks\n\n");
        for (i, chunk) in doc.chunks.iter().enumerate() {
            md.push_str(&format!("### Chunk {i}\n\n"));
            md.push_str(&format!("{}\n\n", chunk.text));
        }
    }

    fs::write(output.join("documents").join(format!("{}.md", doc.id)), md)?;
    Ok(())
}

fn write_wiki_file(dir: &Path, page: &WikiPage, entities: &[Entity]) -> anyhow::Result<()> {
    let mut md = String::new();

    md.push_str("---\n");
    md.push_str(&format!("slug: \"{}\"\n", page.slug));
    md.push_str(&format!("category: \"{}\"\n", page.category));
    md.push_str("---\n\n");

    md.push_str(&format!("# {}\n\n", page.title));

    if !page.summary.is_empty() {
        md.push_str(&format!("{}\n\n", page.summary));
    }

    md.push_str(&format!("{}\n\n", page.content));

    // Backlinks: entities whose slug matches this page's slug
    let backlinks: Vec<&Entity> = entities
        .iter()
        .filter(|e| slugify(&e.name) == page.slug)
        .collect();
    if !backlinks.is_empty() {
        md.push_str("## Backlinks\n\n");
        for entity in &backlinks {
            let entity_slug = slugify(&entity.name);
            md.push_str(&format!("- [[entities/{entity_slug}|{}]]\n", entity.name,));
        }
        md.push('\n');
    }

    fs::write(dir.join(format!("{}.md", page.slug)), md)?;
    Ok(())
}

fn write_index_file(
    output: &Path,
    entities: &[Entity],
    edges: &[Relation],
    ctx: &ExportContext,
    wiki_pages: &[WikiPage],
) -> anyhow::Result<()> {
    let doc_count = ctx.docs.list().count();
    let mut md = String::new();

    md.push_str("# kgx Export\n\n");
    md.push_str(&format!("- Entities: {}\n", entities.len()));
    md.push_str(&format!("- Relations: {}\n", edges.len()));
    md.push_str(&format!("- Documents: {doc_count}\n"));
    md.push_str(&format!("- Wiki Pages: {}\n\n", wiki_pages.len()));

    if !entities.is_empty() {
        md.push_str("## Entities\n\n");
        let mut sorted: Vec<&Entity> = entities.iter().collect();
        sorted.sort_by_key(|e| &e.name);
        for entity in &sorted {
            let slug = slugify(&entity.name);
            md.push_str(&format!(
                "- [[entities/{slug}|{}]] ({})\n",
                entity.name, entity.entity_type,
            ));
        }
        md.push('\n');
    }

    let docs: Vec<_> = ctx.docs.list().collect();
    if !docs.is_empty() {
        md.push_str("## Documents\n\n");
        for doc in &docs {
            md.push_str(&format!("- [[documents/{}|{}]]\n", doc.id, doc.title));
        }
        md.push('\n');
    }

    if !wiki_pages.is_empty() {
        md.push_str("## Wiki Pages\n\n");
        for page in wiki_pages {
            md.push_str(&format!(
                "- [[wiki/{}/{}|{}]]\n",
                page.category.as_dir(),
                page.slug,
                page.title,
            ));
        }
        md.push('\n');
    }

    fs::write(output.join("index.md"), md)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::WikiCategory;
    use crate::{DocumentStore, EdgeInput, GraphStore, WikiStore};
    use uuid::Uuid;

    fn test_dir() -> String {
        format!("/tmp/kgx_test_md_export_{}", Uuid::new_v4())
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
    fn markdown_export_creates_structure() {
        let dir = test_dir();
        let (graph, docs, wiki) = setup_stores(&dir);
        let ctx = ExportContext {
            graph: &graph,
            docs: &docs,
            wiki: &wiki,
        };
        let out = format!("{dir}/output");
        MarkdownExporter
            .export(&ctx, Path::new(&out))
            .expect("export");

        assert!(Path::new(&format!("{out}/entities/rust.md")).exists());
        assert!(Path::new(&format!("{out}/entities/cargo.md")).exists());
        assert!(Path::new(&format!("{out}/documents/d1.md")).exists());
        assert!(Path::new(&format!("{out}/wiki/entity/rust.md")).exists());
        assert!(Path::new(&format!("{out}/index.md")).exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn entity_file_has_frontmatter_and_relations() {
        let dir = test_dir();
        let (graph, docs, wiki) = setup_stores(&dir);
        let ctx = ExportContext {
            graph: &graph,
            docs: &docs,
            wiki: &wiki,
        };
        let out = format!("{dir}/output");
        MarkdownExporter
            .export(&ctx, Path::new(&out))
            .expect("export");

        let content = std::fs::read_to_string(format!("{out}/entities/rust.md")).expect("read");
        assert!(content.starts_with("---\n"));
        assert!(content.contains("type: language"));
        assert!(content.contains("# Rust"));
        assert!(content.contains("Systems language"));
        assert!(content.contains("[[entities/cargo|Cargo]]"));
        assert!(content.contains("[[documents/d1|Intro to Rust]]"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn document_file_has_chunks() {
        let dir = test_dir();
        let (graph, docs, wiki) = setup_stores(&dir);
        let ctx = ExportContext {
            graph: &graph,
            docs: &docs,
            wiki: &wiki,
        };
        let out = format!("{dir}/output");
        MarkdownExporter
            .export(&ctx, Path::new(&out))
            .expect("export");

        let content = std::fs::read_to_string(format!("{out}/documents/d1.md")).expect("read");
        assert!(content.contains("# Intro to Rust"));
        assert!(content.contains("Rust is great."));
        assert!(content.contains("## Chunks"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn index_file_has_stats_and_links() {
        let dir = test_dir();
        let (graph, docs, wiki) = setup_stores(&dir);
        let ctx = ExportContext {
            graph: &graph,
            docs: &docs,
            wiki: &wiki,
        };
        let out = format!("{dir}/output");
        MarkdownExporter
            .export(&ctx, Path::new(&out))
            .expect("export");

        let content = std::fs::read_to_string(format!("{out}/index.md")).expect("read");
        assert!(content.contains("# kgx Export"));
        assert!(content.contains("Entities: 2"));
        assert!(content.contains("Relations: 1"));
        assert!(content.contains("Documents: 1"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
