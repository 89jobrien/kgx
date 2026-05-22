use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "kgx", about = "Knowledge graph toolkit")]
pub struct Cli {
    /// Root directory for all kgx data (default: .kgx in cwd)
    #[arg(long, default_value = ".kgx")]
    pub root: PathBuf,

    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Subcommand)]
pub enum Cmd {
    /// Initialize a new kgx workspace
    Init,
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
pub enum GraphCmd {
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
pub enum WikiCmd {
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
pub enum DocsCmd {
    /// List all ingested documents
    List,
    /// Search chunks by keyword
    Search { query: String },
}
