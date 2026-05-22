use std::fs;
use std::path::Path;

use anyhow::{Context, bail};

/// Bootstrap a kgx workspace at the given root directory.
///
/// Creates:
/// - `<root>/data/graph.json` (empty graph)
/// - `<root>/data/documents.json` (empty document store)
/// - `<root>/wiki/{summary,entity,topic}/` (empty wiki dirs)
///
/// Fails if the root directory already exists.
pub fn init_workspace(root: &Path) -> anyhow::Result<()> {
    if root.exists() {
        bail!(
            "directory already exists: {}. Remove it first or use a different path.",
            root.display()
        );
    }

    let data_dir = root.join("data");
    fs::create_dir_all(&data_dir).with_context(|| format!("creating {}", data_dir.display()))?;

    for cat in ["summary", "entity", "topic"] {
        fs::create_dir_all(root.join("wiki").join(cat))
            .with_context(|| format!("creating wiki/{cat}"))?;
    }

    let empty_graph = r#"{"nodes":[],"edges":[]}"#;
    let empty_docs = r#"{"documents":[]}"#;

    fs::write(data_dir.join("graph.json"), empty_graph).context("writing graph.json")?;
    fs::write(data_dir.join("documents.json"), empty_docs).context("writing documents.json")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_creates_structure() {
        let dir = format!("/tmp/kgx_init_test_{}", uuid::Uuid::new_v4());
        let root = Path::new(&dir);

        init_workspace(root).expect("init should succeed");

        assert!(root.join("data/graph.json").is_file());
        assert!(root.join("data/documents.json").is_file());
        assert!(root.join("wiki/summary").is_dir());
        assert!(root.join("wiki/entity").is_dir());
        assert!(root.join("wiki/topic").is_dir());

        // Verify the JSON files are valid and loadable.
        let graph =
            crate::GraphStore::open(root.join("data/graph.json")).expect("graph should open");
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);

        let docs =
            crate::DocumentStore::open(root.join("data/documents.json")).expect("docs should open");
        assert_eq!(docs.doc_count(), 0);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn init_fails_if_exists() {
        let dir = format!("/tmp/kgx_init_test_{}", uuid::Uuid::new_v4());
        let root = Path::new(&dir);
        fs::create_dir_all(root).expect("setup should work");

        let result = init_workspace(root);
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("already exists"),
            "error should mention 'already exists'"
        );

        let _ = std::fs::remove_dir_all(root);
    }
}
