use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use crate::types::{LintReport, WikiCategory, WikiPage};

const CATEGORIES: &[&str] = &["summary", "entity", "topic"];

/// Max characters in a search result snippet.
const SNIPPET_MAX_LEN: usize = 120;

/// Manages a directory of markdown wiki pages with cross-references.
#[derive(Debug)]
pub struct WikiStore {
    root: PathBuf,
}

impl WikiStore {
    pub fn open(root: impl Into<PathBuf>) -> anyhow::Result<Self> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        for cat in CATEGORIES {
            fs::create_dir_all(root.join(cat))?;
        }
        Ok(Self { root })
    }

    /// Write or overwrite a wiki page.
    pub fn write_page(
        &self,
        category: WikiCategory,
        title: &str,
        content: &str,
        summary: &str,
    ) -> anyhow::Result<WikiPage> {
        let slug = slugify(title);
        let dir = self.root.join(category.as_dir());
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{slug}.md"));
        fs::write(&path, content)?;

        let page = WikiPage {
            slug,
            category,
            title: title.to_string(),
            content: content.to_string(),
            summary: summary.to_string(),
        };

        self.update_index()?;
        self.append_log(&format!("write: {}", page.title))?;
        Ok(page)
    }

    /// Read a wiki page by category and title.
    pub fn read_page(&self, category: WikiCategory, title: &str) -> anyhow::Result<Option<String>> {
        let slug = slugify(title);
        let path = self.root.join(category.as_dir()).join(format!("{slug}.md"));
        if path.exists() {
            Ok(Some(fs::read_to_string(path)?))
        } else {
            Ok(None)
        }
    }

    /// Keyword search across all wiki pages. Returns (slug, category_dir, snippet).
    pub fn search(&self, query: &str) -> anyhow::Result<Vec<WikiSearchHit>> {
        let q = query.to_lowercase();
        let mut hits = Vec::new();
        for cat in CATEGORIES {
            let dir = self.root.join(cat);
            if !dir.exists() {
                continue;
            }
            for entry in fs::read_dir(&dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "md") {
                    let content = fs::read_to_string(&path)?;
                    if content.to_lowercase().contains(&q) {
                        let slug = path
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        let snippet = extract_snippet(&content, &q, SNIPPET_MAX_LEN);
                        hits.push(WikiSearchHit {
                            slug,
                            category: cat.to_string(),
                            path: path.clone(),
                            snippet,
                        });
                    }
                }
            }
        }
        Ok(hits)
    }

    /// List all pages in a category.
    pub fn list_pages(&self, category: WikiCategory) -> anyhow::Result<Vec<String>> {
        let dir = self.root.join(category.as_dir());
        if !dir.exists() {
            return Ok(vec![]);
        }
        let mut slugs = Vec::new();
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            if entry.path().extension().is_some_and(|e| e == "md") {
                let slug = entry
                    .path()
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                slugs.push(slug);
            }
        }
        slugs.sort();
        Ok(slugs)
    }

    /// Rebuild the wiki index.md from all existing pages.
    fn update_index(&self) -> anyhow::Result<()> {
        let mut lines = vec!["# Wiki Index\n".to_string()];
        for cat in CATEGORIES {
            let dir = self.root.join(cat);
            if !dir.exists() {
                continue;
            }
            let mut entries: Vec<String> = Vec::new();
            for entry in fs::read_dir(&dir)? {
                let entry = entry?;
                if entry.path().extension().is_some_and(|e| e == "md") {
                    let slug = entry
                        .path()
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    entries.push(format!("- [{slug}]({cat}/{slug}.md)"));
                }
            }
            if !entries.is_empty() {
                entries.sort();
                lines.push(format!("\n## {}\n", capitalize(cat)));
                lines.extend(entries);
            }
        }
        lines.push(String::new());
        fs::write(self.root.join("index.md"), lines.join("\n"))?;
        Ok(())
    }

    fn append_log(&self, msg: &str) -> anyhow::Result<()> {
        use std::io::Write;
        let path = self.root.join("log.md");
        let mut f = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        writeln!(f, "- {msg}")?;
        Ok(())
    }

    /// Lint the wiki for broken wikilinks, orphan pages, etc.
    pub fn lint(&self) -> anyhow::Result<LintReport> {
        let pages = self.iter_pages()?;
        Ok(build_lint_report(&pages))
    }

    /// Iterate over all wiki pages, yielding (slug, content) pairs.
    fn iter_pages(&self) -> anyhow::Result<Vec<(String, String)>> {
        let mut pages = Vec::new();
        for cat in CATEGORIES {
            let dir = self.root.join(cat);
            if !dir.exists() {
                continue;
            }
            for entry in fs::read_dir(&dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().is_none_or(|e| e != "md") {
                    continue;
                }
                let slug = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let content = fs::read_to_string(&path)?;
                pages.push((slug, content));
            }
        }
        Ok(pages)
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct WikiSearchHit {
    pub slug: String,
    pub category: String,
    pub path: PathBuf,
    pub snippet: String,
}

fn slugify(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

/// Extract `[[wikilink]]` targets from markdown content.
fn extract_wikilinks(content: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut rest = content;
    while let Some(start) = rest.find("[[") {
        rest = &rest[start + 2..];
        if let Some(end) = rest.find("]]") {
            let link = &rest[..end];
            links.push(slugify(link));
            rest = &rest[end + 2..];
        } else {
            break;
        }
    }
    links
}

// qual:allow(iosp) reason: "pure logic calling extract_wikilinks — acceptable"
fn build_lint_report(pages: &[(String, String)]) -> LintReport {
    let mut report = LintReport::default();
    let all_slugs: HashSet<String> = pages.iter().map(|(s, _)| s.clone()).collect();
    let mut referenced_slugs = HashSet::new();

    for (slug, content) in pages {
        let links = extract_wikilinks(content);
        if links.is_empty() {
            report.isolated_pages.push(slug.clone());
        }
        for link in links {
            referenced_slugs.insert(link.clone());
            if !all_slugs.contains(&link) {
                report.broken_wikilinks.push((slug.clone(), link));
            }
        }
    }

    for slug in &referenced_slugs {
        if !all_slugs.contains(slug) {
            report.missing_pages.push(slug.clone());
        }
    }
    for slug in &all_slugs {
        if !referenced_slugs.contains(slug) {
            report.orphan_pages.push(slug.clone());
        }
    }

    report.orphan_pages.sort();
    report.missing_pages.sort();
    report.isolated_pages.sort();
    report
}

fn extract_snippet(content: &str, query: &str, max_len: usize) -> String {
    let lower = content.to_lowercase();
    if let Some(byte_pos) = lower.find(query) {
        // Map byte offset in lowercased string back to a char offset,
        // then use char-based slicing on the original content.
        let char_pos = lower[..byte_pos].chars().count();
        let query_char_len = query.chars().count();
        let total_chars = content.chars().count();

        let start = char_pos.saturating_sub(max_len / 2);
        let end = (char_pos + query_char_len + max_len / 2).min(total_chars);
        let slice: String = content.chars().skip(start).take(end - start).collect();
        if start > 0 {
            format!("...{slice}...")
        } else {
            format!("{slice}...")
        }
    } else {
        content.chars().take(max_len).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn slugify_works() {
        assert_eq!(slugify("Memory Leak"), "memory-leak");
        assert_eq!(slugify("foo--bar  baz"), "foo-bar-baz");
    }

    #[test]
    fn slugify_empty() {
        assert_eq!(slugify(""), "");
        assert_eq!(slugify("---"), "");
    }

    #[test]
    fn extract_wikilinks_works() {
        let content = "See [[Memory Leak]] and [[system-crash]].";
        let links = extract_wikilinks(content);
        assert_eq!(links, vec!["memory-leak", "system-crash"]);
    }

    #[test]
    fn extract_wikilinks_unclosed() {
        let content = "See [[broken link and nothing else";
        let links = extract_wikilinks(content);
        assert!(links.is_empty());
    }

    #[test]
    fn extract_snippet_ascii() {
        let content = "The quick brown fox jumps over the lazy dog";
        let snippet = extract_snippet(content, "fox", 20);
        assert!(snippet.contains("fox"));
    }

    /// Regression: extract_snippet used byte offsets from lowercased string
    /// on the original, which panics on multi-byte UTF-8.
    #[test]
    fn extract_snippet_multibyte_utf8() {
        let content = "Ubersicht uber die Straße und Brucke";
        let snippet = extract_snippet(content, "straße", 30);
        assert!(
            snippet.to_lowercase().contains("straße"),
            "snippet should contain the query: {snippet}"
        );
    }

    fn fresh_wiki() -> (WikiStore, String) {
        let dir = format!("/tmp/kgx_wiki_test_{}", uuid::Uuid::new_v4());
        let wiki = WikiStore::open(&dir).expect("wiki should open");
        (wiki, dir)
    }

    #[test]
    fn read_page_roundtrip() {
        let (wiki, dir) = fresh_wiki();
        wiki.write_page(WikiCategory::Entity, "Rust", "Rust content", "summary")
            .expect("write should work");
        let content = wiki
            .read_page(WikiCategory::Entity, "Rust")
            .expect("read should work");
        assert_eq!(content, Some("Rust content".to_string()));

        let missing = wiki
            .read_page(WikiCategory::Entity, "Nonexistent")
            .expect("read should work");
        assert!(missing.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_pages_returns_sorted_slugs() {
        let (wiki, dir) = fresh_wiki();
        wiki.write_page(WikiCategory::Topic, "Zebra", "z", "z")
            .expect("write should work");
        wiki.write_page(WikiCategory::Topic, "Apple", "a", "a")
            .expect("write should work");
        let pages = wiki
            .list_pages(WikiCategory::Topic)
            .expect("list should work");
        assert_eq!(pages, vec!["apple", "zebra"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn lint_finds_broken_wikilinks() {
        let (wiki, dir) = fresh_wiki();
        wiki.write_page(WikiCategory::Entity, "Rust", "See [[NonExistent]].", "test")
            .expect("write should succeed");
        let report = wiki.lint().expect("lint should succeed");
        assert!(
            report
                .broken_wikilinks
                .iter()
                .any(|(_, target)| target == "nonexistent"),
            "should find broken wikilink: {:?}",
            report.broken_wikilinks
        );
        assert!(
            report.missing_pages.contains(&"nonexistent".to_string()),
            "should list missing page"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn build_lint_report_detects_issues() {
        let pages = vec![
            ("rust".to_string(), "See [[missing-page]].".to_string()),
            ("orphan".to_string(), "No links here.".to_string()),
        ];
        let report = build_lint_report(&pages);
        assert!(
            report
                .broken_wikilinks
                .iter()
                .any(|(_, t)| t == "missing-page")
        );
        assert!(report.missing_pages.contains(&"missing-page".to_string()));
        assert!(report.orphan_pages.contains(&"orphan".to_string()));
        assert!(report.isolated_pages.contains(&"orphan".to_string()));
    }

    // Property: slugify is idempotent.
    proptest! {
        #[test]
        fn slugify_idempotent(input in "\\PC{1,80}") {
            let once = slugify(&input);
            let twice = slugify(&once);
            prop_assert_eq!(&once, &twice, "slugify should be idempotent");
        }

        #[test]
        fn slugify_no_consecutive_hyphens(input in "\\PC{1,80}") {
            let slug = slugify(&input);
            prop_assert!(
                !slug.contains("--"),
                "slug should not contain consecutive hyphens: {}", slug
            );
        }

        #[test]
        fn extract_wikilinks_never_panics(input in "\\PC{0,500}") {
            // Just assert it doesn't panic on arbitrary input.
            let _ = extract_wikilinks(&input);
        }
    }
}
