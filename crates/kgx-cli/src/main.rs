use std::io::Read as _;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use kgx::{DocumentStore, GraphStore, WikiCategory, WikiStore};

#[derive(Parser)]
#[command(name = "kgx", about = "Knowledge graph toolkit")]
struct Cli {
    /// Root directory for all kgx data (default: .kgx in cwd)
    #[arg(long, default_value = ".kgx")]
    root: PathBuf,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Ingest a document with entities and relations (JSON on stdin)
    Ingest,
    /// Query the graph by seed entity name
    Query {
        /// Entity name to start BFS from
        name: String,
    },
    /// Graph operations
    #[command(subcommand)]
    Graph(GraphCmd),
    /// Wiki operations
    #[command(subcommand)]
    Wiki(WikiCmd),
    /// Document store operations
    #[command(subcommand)]
    Docs(DocsCmd),
    /// Show stats
    Stats,
}

#[derive(Subcommand)]
enum GraphCmd {
    /// Add a node
    AddNode {
        name: String,
        #[arg(long, alias = "type")]
        entity_type: String,
        #[arg(long)]
        supporting_text: Option<String>,
        #[arg(long)]
        source_doc: Option<String>,
    },
    /// Add an edge
    AddEdge {
        source: String,
        target: String,
        #[arg(long, alias = "type")]
        relation_type: String,
        #[arg(long, default_value = "1.0")]
        confidence: f64,
        #[arg(long)]
        supporting_text: Option<String>,
        #[arg(long)]
        source_doc: Option<String>,
    },
    /// Search nodes by keyword
    Search { query: String },
}

#[derive(Subcommand)]
enum WikiCmd {
    /// Write a wiki page (content on stdin)
    Write {
        #[arg(long)]
        category: String,
        #[arg(long)]
        title: String,
        #[arg(long)]
        summary: String,
    },
    /// Read a wiki page
    Read {
        #[arg(long)]
        category: String,
        #[arg(long)]
        title: String,
    },
    /// Search wiki pages
    Search { query: String },
    /// List pages in a category
    List {
        #[arg(long)]
        category: String,
    },
    /// Lint the wiki for issues
    Lint,
}

#[derive(Subcommand)]
enum DocsCmd {
    /// List all ingested documents
    List,
    /// Search chunks by keyword
    Search { query: String },
}

fn parse_category(s: &str) -> Result<WikiCategory> {
    match s {
        "summary" => Ok(WikiCategory::Summary),
        "entity" => Ok(WikiCategory::Entity),
        "topic" => Ok(WikiCategory::Topic),
        _ => anyhow::bail!("unknown category: {s} (expected summary|entity|topic)"),
    }
}

fn graph_path(root: &PathBuf) -> PathBuf {
    root.join("data").join("graph.json")
}

fn docs_path(root: &PathBuf) -> PathBuf {
    root.join("data").join("documents.json")
}

fn wiki_path(root: &PathBuf) -> PathBuf {
    root.join("wiki")
}

fn read_stdin() -> Result<String> {
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .context("reading stdin")?;
    Ok(buf)
}

/// JSON input format for the `ingest` command.
#[derive(serde::Deserialize)]
struct IngestInput {
    doc_id: String,
    title: String,
    source: String,
    raw_content: String,
    #[serde(default)]
    entities: Vec<IngestEntity>,
    #[serde(default)]
    relations: Vec<IngestRelation>,
}

#[derive(serde::Deserialize)]
struct IngestEntity {
    name: String,
    #[serde(rename = "type")]
    entity_type: String,
    supporting_text: Option<String>,
}

#[derive(serde::Deserialize)]
struct IngestRelation {
    source: String,
    target: String,
    #[serde(rename = "type")]
    relation_type: String,
    #[serde(default = "default_confidence")]
    confidence: f64,
    supporting_text: Option<String>,
}

fn default_confidence() -> f64 {
    1.0
}

#[derive(serde::Serialize)]
struct IngestOutput {
    doc_id: String,
    chunk_count: usize,
    nodes_added: usize,
    edges_added: usize,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = &cli.root;

    match cli.cmd {
        Cmd::Ingest => {
            let input: IngestInput =
                serde_json::from_str(&read_stdin()?).context("parsing ingest JSON from stdin")?;

            let mut graph = GraphStore::open(graph_path(root))?;
            let mut docs = DocumentStore::open(docs_path(root))?;

            let doc = docs.ingest(
                &input.doc_id,
                &input.title,
                &input.source,
                &input.raw_content,
            );
            let chunk_count = doc.chunks.len();

            let mut nodes_added = 0usize;
            for e in &input.entities {
                graph.add_node(
                    &e.name,
                    &e.entity_type,
                    e.supporting_text.as_deref(),
                    Some(&input.doc_id),
                );
                nodes_added += 1;
            }

            let mut edges_added = 0usize;
            for r in &input.relations {
                let src = graph
                    .node_by_name(&r.source)
                    .context(format!("unknown source entity: {}", r.source))?;
                let tgt = graph
                    .node_by_name(&r.target)
                    .context(format!("unknown target entity: {}", r.target))?;
                if graph
                    .add_edge(
                        src,
                        tgt,
                        &r.relation_type,
                        r.confidence,
                        r.supporting_text.as_deref(),
                        Some(&input.doc_id),
                    )
                    .is_some()
                {
                    edges_added += 1;
                }
            }

            graph.save()?;
            docs.save()?;

            let out = IngestOutput {
                doc_id: input.doc_id,
                chunk_count,
                nodes_added,
                edges_added,
            };
            println!("{}", serde_json::to_string_pretty(&out)?);
        }

        Cmd::Query { name } => {
            let graph = GraphStore::open(graph_path(root))?;
            let docs = DocumentStore::open(docs_path(root))?;

            let seed = graph
                .node_by_name(&name)
                .context(format!("entity not found: {name}"))?;
            let (nodes, edges) = graph.bfs_subgraph(seed);

            // Collect supporting chunks from related docs.
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
                query: name,
                nodes,
                edges,
                supporting_chunks: chunks.into_iter().cloned().collect(),
            };
            println!("{}", serde_json::to_string_pretty(&result)?);
        }

        Cmd::Graph(sub) => {
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
                    let id = graph.add_edge(
                        src,
                        tgt,
                        &relation_type,
                        confidence,
                        supporting_text.as_deref(),
                        source_doc.as_deref(),
                    );
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
                    let results: Vec<_> = graph.search(&query).into_iter().collect();
                    println!("{}", serde_json::to_string_pretty(&results)?);
                }
            }
        }

        Cmd::Wiki(sub) => {
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
        }

        Cmd::Docs(sub) => {
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
        }

        Cmd::Stats => {
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
        }
    }

    Ok(())
}
