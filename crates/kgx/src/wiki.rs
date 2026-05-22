use std::fs;
use std::path::PathBuf;

use crate::types::*;

/// Manages a directory of markdown wiki pages with cross-references.
#[derive(Debug)]
pub struct WikiStore {
    root: PathBuf,
}

impl WikiStore {
    pub fn open(root: impl Into<PathBuf>) -> anyhow::Result<Self> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        for cat in ["summary", "entity", "topic"] {
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
        for cat in ["summary", "entity", "topic"] {
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
                        let snippet = extract_snippet(&content, &q, 120);
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
        for cat in ["summary", "entity", "topic"] {
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
        let mut report = LintReport::default();
        let mut all_slugs: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut referenced_slugs: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        let mut pages_with_links: std::collections::HashSet<String> =
            std::collections::HashSet::new();

        // Collect all slugs and wikilinks.
        for cat in ["summary", "entity", "topic"] {
            let dir = self.root.join(cat);
            if !dir.exists() {
                continue;
            }
            for entry in fs::read_dir(&dir)? {
                let entry = entry?;
                let path = entry.path();
                if !path.extension().is_some_and(|e| e == "md") {
                    continue;
                }
                let slug = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                all_slugs.insert(slug.clone());

                let content = fs::read_to_string(&path)?;
                let links = extract_wikilinks(&content);
                if links.is_empty() {
                    report.isolated_pages.push(slug.clone());
                }
                for link in &links {
                    pages_with_links.insert(slug.clone());
                    referenced_slugs.insert(link.clone());
                    if !all_slugs.contains(link) {
                        // Could be forward-ref; we'll check after full scan.
                    }
                }
            }
        }

        // Broken wikilinks: referenced but don't exist.
        for slug in &referenced_slugs {
            if !all_slugs.contains(slug) {
                report.missing_pages.push(slug.clone());
            }
        }

        // Orphan pages: exist but never referenced.
        for slug in &all_slugs {
            if !referenced_slugs.contains(slug) {
                report.orphan_pages.push(slug.clone());
            }
        }

        report.orphan_pages.sort();
        report.missing_pages.sort();
        report.isolated_pages.sort();
        Ok(report)
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

fn extract_snippet(content: &str, query: &str, max_len: usize) -> String {
    let lower = content.to_lowercase();
    if let Some(pos) = lower.find(query) {
        let start = pos.saturating_sub(max_len / 2);
        let end = (pos + query.len() + max_len / 2).min(content.len());
        let slice = &content[start..end];
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

    #[test]
    fn slugify_works() {
        assert_eq!(slugify("Memory Leak"), "memory-leak");
        assert_eq!(slugify("foo--bar  baz"), "foo-bar-baz");
    }

    #[test]
    fn extract_wikilinks_works() {
        let content = "See [[Memory Leak]] and [[system-crash]].";
        let links = extract_wikilinks(content);
        assert_eq!(links, vec!["memory-leak", "system-crash"]);
    }
}
