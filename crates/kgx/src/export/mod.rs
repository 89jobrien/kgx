pub mod gfm;
pub mod json;
pub mod markdown;

use std::path::Path;

use crate::{DocumentStore, GraphStore, WikiStore};

/// All three stores bundled for export.
pub struct ExportContext<'a> {
    pub graph: &'a GraphStore,
    pub docs: &'a DocumentStore,
    pub wiki: &'a WikiStore,
}

/// Format-agnostic export interface.
pub trait Exporter {
    fn export(&self, ctx: &ExportContext, output: &Path) -> anyhow::Result<()>;
}

pub use gfm::GfmExporter;
pub use json::JsonExporter;
pub use markdown::MarkdownExporter;

/// Conformance test suite for the Exporter trait.
/// Any implementation must:
/// 1. Succeed on an empty context (no panic, no error)
/// 2. Succeed on a populated context
/// 3. Create the output directory if it doesn't exist
/// 4. Write at least one file to the output directory
#[cfg(test)]
pub mod conformance {
    use super::*;
    use crate::types::WikiCategory;
    use crate::{DocumentStore, EdgeInput, GraphStore, WikiStore};

    pub fn assert_exporter_contract(exporter: &dyn Exporter, label: &str) {
        let dir = format!("/tmp/kgx_conformance_{}_{}", label, uuid::Uuid::new_v4());

        // 1. Empty context succeeds
        {
            let graph =
                GraphStore::open(format!("{dir}/empty/graph.json")).expect("graph should open");
            let docs =
                DocumentStore::open(format!("{dir}/empty/docs.json")).expect("docs should open");
            let wiki = WikiStore::open(format!("{dir}/empty/wiki")).expect("wiki should open");
            let ctx = ExportContext {
                graph: &graph,
                docs: &docs,
                wiki: &wiki,
            };
            let out = format!("{dir}/empty/output");
            exporter
                .export(&ctx, Path::new(&out))
                .expect("export of empty context should succeed");
            assert!(
                Path::new(&out).exists(),
                "{label}: output dir should be created for empty context"
            );
        }

        // 2. Populated context succeeds and writes files
        {
            let mut graph =
                GraphStore::open(format!("{dir}/pop/graph.json")).expect("graph should open");
            let a = graph.add_node("Alpha", "concept", Some("first"), Some("d1"));
            let b = graph.add_node("Beta", "concept", Some("second"), Some("d1"));
            graph.add_edge(EdgeInput {
                source: a,
                target: b,
                relation_type: "precedes",
                confidence: 0.8,
                supporting_text: Some("Alpha before Beta"),
                source_doc: Some("d1"),
            });
            let mut docs =
                DocumentStore::open(format!("{dir}/pop/docs.json")).expect("docs should open");
            docs.ingest("d1", "Test Document", "test.md", "Alpha and Beta content.");
            let wiki = WikiStore::open(format!("{dir}/pop/wiki")).expect("wiki should open");
            wiki.write_page(WikiCategory::Entity, "Alpha", "Alpha page", "Alpha summary")
                .expect("wiki write should succeed");

            let ctx = ExportContext {
                graph: &graph,
                docs: &docs,
                wiki: &wiki,
            };
            let out = format!("{dir}/pop/output");
            exporter
                .export(&ctx, Path::new(&out))
                .expect("export of populated context should succeed");

            // Must write at least one file
            let files: Vec<_> = std::fs::read_dir(&out)
                .expect("output dir should be readable")
                .filter_map(|e| e.ok())
                .collect();
            assert!(
                !files.is_empty(),
                "{label}: exporter should write at least one file"
            );
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn json_exporter_satisfies_contract() {
        assert_exporter_contract(&JsonExporter, "json");
    }

    #[test]
    fn markdown_exporter_satisfies_contract() {
        assert_exporter_contract(&MarkdownExporter, "markdown");
    }

    #[test]
    fn gfm_exporter_satisfies_contract() {
        assert_exporter_contract(&GfmExporter, "gfm");
    }
}
