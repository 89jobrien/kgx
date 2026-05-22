/// JSON input format for the `ingest` command.
#[derive(serde::Deserialize)]
pub struct IngestInput {
    pub doc_id: String,
    pub title: String,
    pub source: String,
    pub raw_content: String,
    #[serde(default)]
    pub entities: Vec<IngestEntity>,
    #[serde(default)]
    pub relations: Vec<IngestRelation>,
}

#[derive(serde::Deserialize)]
pub struct IngestEntity {
    pub name: String,
    #[serde(rename = "type")]
    pub entity_type: String,
    pub supporting_text: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct IngestRelation {
    pub source: String,
    pub target: String,
    #[serde(rename = "type")]
    pub relation_type: String,
    #[serde(default = "default_confidence")]
    pub confidence: f64,
    pub supporting_text: Option<String>,
}

fn default_confidence() -> f64 {
    1.0
}

#[derive(serde::Serialize)]
pub struct IngestOutput {
    pub doc_id: String,
    pub chunk_count: usize,
    pub nodes_added: usize,
    pub edges_added: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_confidence_is_one() {
        assert_eq!(default_confidence(), 1.0);
    }

    #[test]
    fn ingest_relation_defaults_confidence() {
        let json = r#"{"source":"A","target":"B","type":"rel"}"#;
        let r: IngestRelation = serde_json::from_str(json).expect("should parse");
        assert_eq!(r.confidence, 1.0);
    }
}
