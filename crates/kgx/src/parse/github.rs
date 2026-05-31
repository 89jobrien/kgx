use crate::parse::{ParseError, ParsedDocument, ParsedEntity, ParsedRelation, Parser};

/// Parses raw GitHub API JSON (from `gh api repos/owner/repo`) into
/// entities and relations.
pub struct GitHubParser;

/// Subset of GitHub API repo response we care about.
#[derive(serde::Deserialize)]
struct GitHubRepo {
    full_name: String,
    description: Option<String>,
    owner: GitHubOwner,
    #[serde(default)]
    topics: Vec<String>,
    language: Option<String>,
    license: Option<GitHubLicense>,
}

#[derive(serde::Deserialize)]
struct GitHubOwner {
    login: String,
    #[serde(rename = "type")]
    owner_type: String,
}

#[derive(serde::Deserialize)]
struct GitHubLicense {
    spdx_id: String,
}

impl Parser for GitHubParser {
    fn parse(&self, input: &str, doc_id: &str) -> Result<ParsedDocument, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParsedDocument {
                doc_id: doc_id.to_string(),
                entities: vec![],
                relations: vec![],
            });
        }

        let repo: GitHubRepo = match serde_json::from_str(input) {
            Ok(r) => r,
            Err(_) => {
                return Ok(ParsedDocument {
                    doc_id: doc_id.to_string(),
                    entities: vec![],
                    relations: vec![],
                });
            }
        };

        let mut entities = Vec::new();
        let mut relations = Vec::new();

        // Repo entity
        entities.push(ParsedEntity {
            name: repo.full_name.clone(),
            entity_type: "repository".to_string(),
            supporting_text: repo.description.clone(),
        });

        // Owner entity
        let owner_type = repo.owner.owner_type.to_lowercase();
        entities.push(ParsedEntity {
            name: repo.owner.login.clone(),
            entity_type: owner_type,
            supporting_text: None,
        });
        relations.push(ParsedRelation {
            source: repo.full_name.clone(),
            target: repo.owner.login.clone(),
            relation_type: "owned_by".to_string(),
            confidence: 1.0,
            supporting_text: None,
        });

        // Topics
        for topic in &repo.topics {
            entities.push(ParsedEntity {
                name: topic.clone(),
                entity_type: "topic".to_string(),
                supporting_text: None,
            });
            relations.push(ParsedRelation {
                source: repo.full_name.clone(),
                target: topic.clone(),
                relation_type: "tagged_with".to_string(),
                confidence: 1.0,
                supporting_text: None,
            });
        }

        // Language
        if let Some(lang) = &repo.language {
            entities.push(ParsedEntity {
                name: lang.clone(),
                entity_type: "language".to_string(),
                supporting_text: None,
            });
            relations.push(ParsedRelation {
                source: repo.full_name.clone(),
                target: lang.clone(),
                relation_type: "written_in".to_string(),
                confidence: 1.0,
                supporting_text: None,
            });
        }

        // License
        if let Some(lic) = &repo.license {
            entities.push(ParsedEntity {
                name: lic.spdx_id.clone(),
                entity_type: "license".to_string(),
                supporting_text: None,
            });
            relations.push(ParsedRelation {
                source: repo.full_name.clone(),
                target: lic.spdx_id.clone(),
                relation_type: "licensed_under".to_string(),
                confidence: 1.0,
                supporting_text: None,
            });
        }

        Ok(ParsedDocument {
            doc_id: doc_id.to_string(),
            entities,
            relations,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::conformance;

    const SAMPLE_REPO: &str = r#"{
        "full_name": "rust-lang/rust",
        "description": "The Rust programming language",
        "owner": { "login": "rust-lang", "type": "Organization" },
        "topics": ["rust", "compiler"],
        "language": "Rust",
        "license": { "spdx_id": "MIT" }
    }"#;

    #[test]
    fn github_parser_extracts_repo_entity() {
        let p = GitHubParser;
        let doc = p.parse(SAMPLE_REPO, "gh-1").unwrap();
        let repo = doc.entities.iter().find(|e| e.entity_type == "repository");
        assert!(repo.is_some());
        assert_eq!(repo.unwrap().name, "rust-lang/rust");
    }

    #[test]
    fn github_parser_extracts_owner() {
        let p = GitHubParser;
        let doc = p.parse(SAMPLE_REPO, "gh-1").unwrap();
        let owner = doc.entities.iter().find(|e| e.name == "rust-lang");
        assert!(owner.is_some());
        assert_eq!(owner.unwrap().entity_type, "organization");
    }

    #[test]
    fn github_parser_extracts_topics() {
        let p = GitHubParser;
        let doc = p.parse(SAMPLE_REPO, "gh-1").unwrap();
        let topics: Vec<_> = doc
            .entities
            .iter()
            .filter(|e| e.entity_type == "topic")
            .map(|e| e.name.as_str())
            .collect();
        assert_eq!(topics, vec!["rust", "compiler"]);
    }

    #[test]
    fn github_parser_extracts_language() {
        let p = GitHubParser;
        let doc = p.parse(SAMPLE_REPO, "gh-1").unwrap();
        let lang = doc.entities.iter().find(|e| e.entity_type == "language");
        assert!(lang.is_some());
        assert_eq!(lang.unwrap().name, "Rust");
    }

    #[test]
    fn github_parser_extracts_license() {
        let p = GitHubParser;
        let doc = p.parse(SAMPLE_REPO, "gh-1").unwrap();
        let lic = doc.entities.iter().find(|e| e.entity_type == "license");
        assert!(lic.is_some());
        assert_eq!(lic.unwrap().name, "MIT");
    }

    #[test]
    fn github_parser_creates_relations() {
        let p = GitHubParser;
        let doc = p.parse(SAMPLE_REPO, "gh-1").unwrap();
        let rel_types: Vec<_> = doc
            .relations
            .iter()
            .map(|r| r.relation_type.as_str())
            .collect();
        assert!(rel_types.contains(&"owned_by"));
        assert!(rel_types.contains(&"tagged_with"));
        assert!(rel_types.contains(&"written_in"));
        assert!(rel_types.contains(&"licensed_under"));
    }

    #[test]
    fn github_parser_satisfies_contract() {
        conformance::assert_parser_contract(&GitHubParser, "github");
    }

    #[test]
    fn github_parser_handles_missing_optional_fields() {
        let json = r#"{
            "full_name": "user/minimal",
            "description": null,
            "owner": { "login": "user", "type": "User" }
        }"#;
        let p = GitHubParser;
        let doc = p.parse(json, "gh-2").unwrap();
        assert!(!doc.entities.is_empty());
    }
}
