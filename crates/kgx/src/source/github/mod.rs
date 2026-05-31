pub mod http;
pub mod octocrab;

use std::process::Command;

use crate::parse::github::GitHubParser;
use crate::parse::{ParsedDocument, Parser};
use crate::source::{Source, SourceError};

/// Controls how much data to extract from a GitHub repo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Layer {
    Metadata,
    Docs,
    Deps,
    Issues,
}

impl Layer {
    /// Returns true if `self` includes the given layer.
    pub fn includes(self, other: Layer) -> bool {
        self >= other
    }
}

impl std::str::FromStr for Layer {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "metadata" => Ok(Layer::Metadata),
            "docs" => Ok(Layer::Docs),
            "deps" => Ok(Layer::Deps),
            "issues" => Ok(Layer::Issues),
            other => Err(format!(
                "unknown layer: {other} (expected metadata, docs, deps, issues)"
            )),
        }
    }
}

/// Fetches GitHub repo data via the `gh` CLI.
pub struct GitHubSource {
    owner_repo: String,
    layer: Layer,
}

impl GitHubSource {
    pub fn new(owner_repo: &str, layer: Layer) -> Self {
        Self {
            owner_repo: owner_repo.to_string(),
            layer,
        }
    }

    fn fetch_repo_json(&self) -> Result<String, SourceError> {
        let output = Command::new("gh")
            .args(["api", &format!("repos/{}", self.owner_repo)])
            .output()
            .map_err(|e| SourceError::FetchFailed(format!("failed to run gh: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SourceError::FetchFailed(format!("gh api failed: {stderr}")));
        }

        String::from_utf8(output.stdout)
            .map_err(|e| SourceError::FetchFailed(format!("invalid utf8: {e}")))
    }
}

impl Source for GitHubSource {
    fn fetch(&self) -> Result<Vec<ParsedDocument>, SourceError> {
        let json = self.fetch_repo_json()?;
        let parser = GitHubParser;
        let doc_id = format!("github:{}", self.owner_repo);
        let doc = parser.parse(&json, &doc_id)?;
        // Layer 1 (Metadata) only for now.
        // Future: if self.layer.includes(Layer::Docs) { ... }
        let _ = self.layer;
        Ok(vec![doc])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_ordering() {
        assert!(Layer::Metadata < Layer::Docs);
        assert!(Layer::Docs < Layer::Deps);
        assert!(Layer::Deps < Layer::Issues);
    }

    #[test]
    fn layer_includes_lower() {
        assert!(Layer::Docs.includes(Layer::Metadata));
        assert!(Layer::Issues.includes(Layer::Deps));
        assert!(!Layer::Metadata.includes(Layer::Docs));
    }

    #[test]
    fn layer_from_str() {
        assert_eq!("metadata".parse::<Layer>().unwrap(), Layer::Metadata);
        assert_eq!("docs".parse::<Layer>().unwrap(), Layer::Docs);
        assert_eq!("deps".parse::<Layer>().unwrap(), Layer::Deps);
        assert_eq!("issues".parse::<Layer>().unwrap(), Layer::Issues);
        assert!("bogus".parse::<Layer>().is_err());
    }
}
