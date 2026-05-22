mod cli;
mod ingest;

use std::io::Read as _;
use std::path::Path;

use anyhow::{Context, Result};
use clap::Parser;
use kgx::{
    DocumentStore, EdgeInput, GraphStore, IngestEntity, IngestRelation, WikiCategory, WikiStore,
    docs_path, graph_path, wiki_path,
};

use cli::{Cli, Cmd, DocsCmd, GraphCmd, WikiCmd};
use ingest::{IngestInput, IngestOutput};

fn parse_category(s: &str) -> Result<WikiCategory> {
    s.parse::<WikiCategory>()
        .map_err(|e| anyhow::anyhow!("{e}"))
}

fn read_stdin() -> Result<String> {
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .context("reading stdin")?;
    Ok(buf)
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = &cli.root;

    match cli.cmd {
        Cmd::Init => cmd_init(root),
        Cmd::Ingest => cmd_ingest(root),
        Cmd::Query { name } => cmd_query(root, &name),
        Cmd::Graph(sub) => cmd_graph(root, sub),
        Cmd::Wiki(sub) => cmd_wiki(root, sub),
        Cmd::Docs(sub) => cmd_docs(root, sub),
        Cmd::Stats => cmd_stats(root),
    }
}

fn cmd_init(root: &Path) -> Result<()> {
    kgx::init_workspace(root)?;
    println!(
        "Initialized kgx workspace at {}",
        root.canonicalize()
            .unwrap_or_else(|_| root.to_path_buf())
            .display()
    );
    Ok(())
}

fn cmd_ingest(root: &Path) -> Result<()> {
    let input: IngestInput =
        serde_json::from_str(&read_stdin()?).context("parsing ingest JSON from stdin")?;

    let mut graph = GraphStore::open(graph_path(root))?;
    let mut docs = DocumentStore::open(docs_path(root))?;

    let chunk_count = docs
        .ingest(
            &input.doc_id,
            &input.title,
            &input.source,
            &input.raw_content,
        )
        .chunks
        .len();
    let entities: Vec<_> = input
        .entities
        .iter()
        .map(|e| IngestEntity {
            name: &e.name,
            entity_type: &e.entity_type,
            supporting_text: e.supporting_text.as_deref(),
        })
        .collect();
    let relations: Vec<_> = input
        .relations
        .iter()
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
    docs.save()?;

    let out = IngestOutput {
        doc_id: input.doc_id,
        chunk_count,
        nodes_added,
        edges_added,
    };
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

fn cmd_query(root: &Path, name: &str) -> Result<()> {
    let graph = GraphStore::open(graph_path(root))?;
    let docs = DocumentStore::open(docs_path(root))?;

    let seed = graph
        .node_by_name(name)
        .context(format!("entity not found: {name}"))?;
    let (nodes, edges) = graph.bfs_subgraph(seed);

    let doc_ids: std::collections::HashSet<&str> = nodes
        .iter()
        .flat_map(|n| n.source_docs.iter().map(|s| s.as_str()))
        .collect();
    let chunks: Vec<_> = doc_ids
        .iter()
        .filter_map(|id| docs.get(id))
        .flat_map(|d| &d.chunks)
        .collect();

    let result = kgx::QueryResult {
        query: name.to_string(),
        nodes,
        edges,
        supporting_chunks: chunks.into_iter().cloned().collect(),
    };
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

fn cmd_graph(root: &Path, sub: GraphCmd) -> Result<()> {
    let mut graph = GraphStore::open(graph_path(root))?;
    match sub {
        GraphCmd::AddNode {
            name,
            entity_type,
            supporting_text,
            source_doc,
        } => {
            let id = graph.add_node(
                &name,
                &entity_type,
                supporting_text.as_deref(),
                source_doc.as_deref(),
            );
            graph.save()?;
            println!("{}", serde_json::json!({"id": id, "name": name}));
        }
        GraphCmd::AddEdge {
            source,
            target,
            relation_type,
            confidence,
            supporting_text,
            source_doc,
        } => {
            let src = graph
                .node_by_name(&source)
                .context(format!("unknown entity: {source}"))?;
            let tgt = graph
                .node_by_name(&target)
                .context(format!("unknown entity: {target}"))?;
            let id = graph.add_edge(EdgeInput {
                source: src,
                target: tgt,
                relation_type: &relation_type,
                confidence,
                supporting_text: supporting_text.as_deref(),
                source_doc: source_doc.as_deref(),
            });
            graph.save()?;
            match id {
                Some(id) => println!("{}", serde_json::json!({"id": id})),
                None => println!(
                    "{}",
                    serde_json::json!({"skipped": true, "reason": "confidence below threshold"})
                ),
            }
        }
        GraphCmd::Search { query } => {
            let results = graph.search(&query);
            println!("{}", serde_json::to_string_pretty(&results)?);
        }
    }
    Ok(())
}

fn cmd_wiki(root: &Path, sub: WikiCmd) -> Result<()> {
    let wiki = WikiStore::open(wiki_path(root))?;
    match sub {
        WikiCmd::Write {
            category,
            title,
            summary,
        } => {
            let cat = parse_category(&category)?;
            let content = read_stdin()?;
            let page = wiki.write_page(cat, &title, &content, &summary)?;
            println!(
                "{}",
                serde_json::json!({"slug": page.slug, "category": category})
            );
        }
        WikiCmd::Read { category, title } => {
            let cat = parse_category(&category)?;
            match wiki.read_page(cat, &title)? {
                Some(content) => print!("{content}"),
                None => {
                    eprintln!("page not found: {category}/{title}");
                    std::process::exit(1);
                }
            }
        }
        WikiCmd::Search { query } => {
            let hits = wiki.search(&query)?;
            println!("{}", serde_json::to_string_pretty(&hits)?);
        }
        WikiCmd::List { category } => {
            let cat = parse_category(&category)?;
            let pages = wiki.list_pages(cat)?;
            println!("{}", serde_json::to_string_pretty(&pages)?);
        }
        WikiCmd::Lint => {
            let report = wiki.lint()?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
    }
    Ok(())
}

fn cmd_docs(root: &Path, sub: DocsCmd) -> Result<()> {
    let docs = DocumentStore::open(docs_path(root))?;
    match sub {
        DocsCmd::List => {
            let list: Vec<_> = docs
                .list()
                .map(|d| {
                    serde_json::json!({
                        "id": d.id,
                        "title": d.title,
                        "source": d.source,
                        "chunks": d.chunks.len(),
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&list)?);
        }
        DocsCmd::Search { query } => {
            let chunks = docs.search_chunks(&query);
            println!("{}", serde_json::to_string_pretty(&chunks)?);
        }
    }
    Ok(())
}

fn cmd_stats(root: &Path) -> Result<()> {
    let graph = GraphStore::open(graph_path(root))?;
    let docs = DocumentStore::open(docs_path(root))?;
    println!(
        "{}",
        serde_json::json!({
            "nodes": graph.node_count(),
            "edges": graph.edge_count(),
            "documents": docs.doc_count(),
        })
    );
    Ok(())
}
