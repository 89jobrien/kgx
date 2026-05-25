use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;

fn kgx() -> Command {
    Command::cargo_bin("kgx").expect("binary should exist")
}

fn temp_root() -> String {
    format!("/tmp/kgx_cli_test_{}", uuid::Uuid::new_v4())
}

fn init_workspace(root: &str) {
    kgx().args(["--root", root, "init"]).assert().success();
}

mod init {
    use super::*;

    #[test]
    fn creates_workspace() {
        let root = temp_root();
        kgx()
            .args(["--root", &root, "init"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Initialized"));
        assert!(std::path::Path::new(&root).join("data/graph.json").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn fails_if_exists() {
        let root = temp_root();
        fs::create_dir_all(&root).unwrap();
        kgx()
            .args(["--root", &root, "init"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("already exists"));
        let _ = fs::remove_dir_all(&root);
    }
}

mod stats {
    use super::*;

    #[test]
    fn on_empty_workspace() {
        let root = temp_root();
        init_workspace(&root);
        let output = kgx().args(["--root", &root, "stats"]).assert().success();
        let stdout = String::from_utf8_lossy(&output.get_output().stdout);
        let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
        assert_eq!(v["nodes"], 0);
        assert_eq!(v["edges"], 0);
        assert_eq!(v["documents"], 0);
        let _ = fs::remove_dir_all(&root);
    }
}

mod graph {
    use super::*;

    #[test]
    fn add_node_and_search() {
        let root = temp_root();
        init_workspace(&root);

        let output = kgx()
            .args([
                "--root", &root, "graph", "add-node", "Rust", "--type", "language",
            ])
            .assert()
            .success();
        let stdout = String::from_utf8_lossy(&output.get_output().stdout);
        let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
        assert_eq!(v["name"], "Rust");

        kgx()
            .args(["--root", &root, "graph", "search", "rust"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Rust"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn add_edge() {
        let root = temp_root();
        init_workspace(&root);

        kgx()
            .args(["--root", &root, "graph", "add-node", "A", "--type", "t"])
            .assert()
            .success();
        kgx()
            .args(["--root", &root, "graph", "add-node", "B", "--type", "t"])
            .assert()
            .success();

        let output = kgx()
            .args([
                "--root",
                &root,
                "graph",
                "add-edge",
                "A",
                "B",
                "--type",
                "rel",
                "--confidence",
                "0.9",
            ])
            .assert()
            .success();
        let stdout = String::from_utf8_lossy(&output.get_output().stdout);
        let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
        assert!(v["id"].is_string());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn add_edge_low_confidence_skipped() {
        let root = temp_root();
        init_workspace(&root);

        kgx()
            .args(["--root", &root, "graph", "add-node", "A", "--type", "t"])
            .assert()
            .success();
        kgx()
            .args(["--root", &root, "graph", "add-node", "B", "--type", "t"])
            .assert()
            .success();

        let output = kgx()
            .args([
                "--root",
                &root,
                "graph",
                "add-edge",
                "A",
                "B",
                "--type",
                "rel",
                "--confidence",
                "0.1",
            ])
            .assert()
            .success();
        let stdout = String::from_utf8_lossy(&output.get_output().stdout);
        let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
        assert_eq!(v["skipped"], true);

        let _ = fs::remove_dir_all(&root);
    }
}

mod ingest {
    use super::*;

    #[test]
    fn roundtrip() {
        let root = temp_root();
        init_workspace(&root);

        let input = serde_json::json!({
            "doc_id": "d1",
            "title": "Test Doc",
            "source": "test.md",
            "raw_content": "Rust is a systems programming language.",
            "entities": [
                {"name": "Rust", "type": "language"},
                {"name": "Systems Programming", "type": "concept"}
            ],
            "relations": [
                {
                    "source": "Rust",
                    "target": "Systems Programming",
                    "type": "is_a",
                    "confidence": 0.95
                }
            ]
        });

        let output = kgx()
            .args(["--root", &root, "ingest"])
            .write_stdin(serde_json::to_string(&input).unwrap())
            .assert()
            .success();
        let stdout = String::from_utf8_lossy(&output.get_output().stdout);
        let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
        assert_eq!(v["nodes_added"], 2);
        assert_eq!(v["edges_added"], 1);

        kgx()
            .args(["--root", &root, "query", "Rust"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Systems Programming"));

        let output = kgx().args(["--root", &root, "stats"]).assert().success();
        let stdout = String::from_utf8_lossy(&output.get_output().stdout);
        let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
        assert_eq!(v["nodes"], 2);
        assert_eq!(v["documents"], 1);

        let _ = fs::remove_dir_all(&root);
    }
}

mod docs {
    use super::*;

    #[test]
    fn list_and_search() {
        let root = temp_root();
        init_workspace(&root);

        let input = serde_json::json!({
            "doc_id": "d1",
            "title": "Memory Doc",
            "source": "mem.md",
            "raw_content": "memory leak causes crash in production"
        });
        kgx()
            .args(["--root", &root, "ingest"])
            .write_stdin(serde_json::to_string(&input).unwrap())
            .assert()
            .success();

        kgx()
            .args(["--root", &root, "docs", "list"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Memory Doc"));

        kgx()
            .args(["--root", &root, "docs", "search", "memory"])
            .assert()
            .success()
            .stdout(predicate::str::contains("memory leak"));

        let _ = fs::remove_dir_all(&root);
    }
}

mod export {
    use super::*;

    fn setup_populated_workspace(root: &str) {
        init_workspace(root);
        let input = serde_json::json!({
            "doc_id": "d1",
            "title": "Test Doc",
            "source": "test.md",
            "raw_content": "Alpha and Beta.",
            "entities": [
                {"name": "Alpha", "type": "concept"},
                {"name": "Beta", "type": "concept"}
            ],
            "relations": [
                {"source": "Alpha", "target": "Beta", "type": "related", "confidence": 0.9}
            ]
        });
        kgx()
            .args(["--root", root, "ingest"])
            .write_stdin(serde_json::to_string(&input).unwrap())
            .assert()
            .success();
    }

    #[test]
    fn export_gfm() {
        let root = temp_root();
        setup_populated_workspace(&root);
        let out = format!("{root}/gfm-out");

        kgx()
            .args([
                "--root", &root, "export", "--format", "gfm", "--output", &out,
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("Exported gfm"));

        let content = fs::read_to_string(format!("{out}/kgx-export.md")).expect("read export");
        assert!(content.contains("# kgx Export"));
        assert!(content.contains("| Entities | 2 |"));
        assert!(content.contains("| Alpha | concept |"));
        assert!(content.contains("| Alpha | related | Beta |"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn export_json() {
        let root = temp_root();
        setup_populated_workspace(&root);
        let out = format!("{root}/json-out");

        kgx()
            .args([
                "--root", &root, "export", "--format", "json", "--output", &out,
            ])
            .assert()
            .success();

        let content = fs::read_to_string(format!("{out}/kgx-export.json")).expect("read");
        let v: serde_json::Value = serde_json::from_str(&content).expect("parse");
        assert_eq!(v["entities"].as_array().unwrap().len(), 2);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn export_markdown() {
        let root = temp_root();
        setup_populated_workspace(&root);
        let out = format!("{root}/md-out");

        kgx()
            .args([
                "--root", &root, "export", "--format", "markdown", "--output", &out,
            ])
            .assert()
            .success();

        assert!(std::path::Path::new(&format!("{out}/index.md")).exists());
        assert!(std::path::Path::new(&format!("{out}/entities")).is_dir());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn export_unknown_format_fails() {
        let root = temp_root();
        init_workspace(&root);

        kgx()
            .args([
                "--root", &root, "export", "--format", "csv", "--output", "/tmp/x",
            ])
            .assert()
            .failure()
            .stderr(predicate::str::contains("unknown format"));

        let _ = fs::remove_dir_all(&root);
    }
}

mod wiki {
    use super::*;

    #[test]
    fn write_read_list_search_lint() {
        let root = temp_root();
        init_workspace(&root);

        let output = kgx()
            .args([
                "--root",
                &root,
                "wiki",
                "write",
                "--category",
                "entity",
                "--title",
                "Rust",
                "--summary",
                "A language",
            ])
            .write_stdin("Rust is a systems language. See [[Memory Safety]].")
            .assert()
            .success();
        let stdout = String::from_utf8_lossy(&output.get_output().stdout);
        let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
        assert_eq!(v["slug"], "rust");

        kgx()
            .args([
                "--root",
                &root,
                "wiki",
                "read",
                "--category",
                "entity",
                "--title",
                "Rust",
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("systems language"));

        kgx()
            .args(["--root", &root, "wiki", "list", "--category", "entity"])
            .assert()
            .success()
            .stdout(predicate::str::contains("rust"));

        kgx()
            .args(["--root", &root, "wiki", "search", "systems"])
            .assert()
            .success()
            .stdout(predicate::str::contains("rust"));

        kgx()
            .args(["--root", &root, "wiki", "lint"])
            .assert()
            .success()
            .stdout(predicate::str::contains("memory-safety"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn read_missing_page_fails() {
        let root = temp_root();
        init_workspace(&root);

        kgx()
            .args([
                "--root",
                &root,
                "wiki",
                "read",
                "--category",
                "entity",
                "--title",
                "nope",
            ])
            .assert()
            .failure()
            .stderr(predicate::str::contains("page not found"));

        let _ = fs::remove_dir_all(&root);
    }
}
