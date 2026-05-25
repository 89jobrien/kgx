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

pub use json::JsonExporter;
pub use markdown::MarkdownExporter;
