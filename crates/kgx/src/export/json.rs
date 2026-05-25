use std::fs;
use std::path::Path;

use serde::Serialize;

use super::{ExportContext, Exporter};
use crate::types::{Document, Entity, Relation, WikiPage};

pub struct JsonExporter;

#[derive(Serialize)]
struct ExportData {
    entities: Vec<Entity>,
    relations: Vec<Relation>,
    documents: Vec<Document>,
    wiki_pages: Vec<WikiPage>,
}

impl Exporter for JsonExporter {
    fn export(&self, ctx: &ExportContext, output: &Path) -> anyhow::Result<()> {
        fs::create_dir_all(output)?;

        let mut entities: Vec<Entity> = ctx.graph.nodes().cloned().collect();
        entities.sort_by(|a, b| a.name.cmp(&b.name));

        let relations = ctx.graph.edges().to_vec();

        let mut documents: Vec<Document> = ctx.docs.list().cloned().collect();
        documents.sort_by(|a, b| a.id.cmp(&b.id));

        let wiki_pages = ctx.wiki.pages()?;

        let data = ExportData {
            entities,
            relations,
            documents,
            wiki_pages,
        };

        let json = serde_json::to_string_pretty(&data)?;
        fs::write(output.join("kgx-export.json"), json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DocumentStore, GraphStore, WikiStore};
    use uuid::Uuid;

    fn test_dir() -> String {
        format!("/tmp/kgx_test_json_export_{}", Uuid::new_v4())
    }

    #[test]
    fn json_export_writes_file() {
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
        JsonExporter.export(&ctx, Path::new(&out)).expect("export");

        let content = std::fs::read_to_string(format!("{out}/kgx-export.json")).expect("read");
        let v: serde_json::Value = serde_json::from_str(&content).expect("parse");
        assert!(v["entities"].is_array());
        assert!(v["relations"].is_array());
        assert!(v["documents"].is_array());
        assert!(v["wiki_pages"].is_array());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn json_export_includes_data() {
        let dir = test_dir();
        let mut graph = GraphStore::open(format!("{dir}/graph.json")).expect("graph");
        graph.add_node("Rust", "language", Some("systems"), None);
        let mut docs = DocumentStore::open(format!("{dir}/docs.json")).expect("docs");
        docs.ingest("d1", "Doc", "src.md", "content");
        let wiki = WikiStore::open(format!("{dir}/wiki")).expect("wiki");

        let ctx = ExportContext {
            graph: &graph,
            docs: &docs,
            wiki: &wiki,
        };
        let out = format!("{dir}/output");
        JsonExporter.export(&ctx, Path::new(&out)).expect("export");

        let content = std::fs::read_to_string(format!("{out}/kgx-export.json")).expect("read");
        let v: serde_json::Value = serde_json::from_str(&content).expect("parse");
        assert_eq!(v["entities"].as_array().unwrap().len(), 1);
        assert_eq!(v["documents"].as_array().unwrap().len(), 1);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
