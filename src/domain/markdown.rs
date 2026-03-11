use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::domain::document::{Frontmatter, MdDoc};

const DSYNC_LINKS_START: &str = "<!-- dsync-links:start -->";
const DSYNC_LINKS_END: &str = "<!-- dsync-links:end -->";

pub(crate) fn read_md(path: &Path) -> Result<MdDoc> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("Could not read {}", path.display()))?;
    let (frontmatter, content) = parse_frontmatter(&raw)?;

    Ok(MdDoc {
        path: path.to_path_buf(),
        frontmatter,
        content,
    })
}

pub(crate) fn write_md(doc: &MdDoc) -> Result<()> {
    let yaml =
        serde_yaml::to_string(&doc.frontmatter).context("Failed to serialize frontmatter")?;
    let yaml_clean = yaml.trim().trim_start_matches("---").trim().to_string();

    let output = if yaml_clean.is_empty() || yaml_clean == "{}" {
        normalize_newlines(&doc.content)
    } else {
        format!(
            "---\n{}\n---\n\n{}",
            yaml_clean,
            normalize_newlines(&doc.content)
        )
    };

    fs::write(&doc.path, output)
        .with_context(|| format!("Failed to write {}", doc.path.display()))?;
    Ok(())
}

pub(crate) fn parse_frontmatter(raw: &str) -> Result<(Frontmatter, String)> {
    if !raw.starts_with("---\n") {
        return Ok((Frontmatter::default(), normalize_newlines(raw)));
    }

    let rest = &raw[4..];
    if let Some(end_idx) = rest.find("\n---\n") {
        let yaml_part = &rest[..end_idx];
        let content_part = &rest[(end_idx + 5)..];
        let frontmatter = if yaml_part.trim().is_empty() {
            Frontmatter::default()
        } else {
            serde_yaml::from_str(yaml_part).context("Invalid frontmatter YAML")?
        };

        return Ok((frontmatter, normalize_newlines(content_part)));
    }

    Ok((Frontmatter::default(), normalize_newlines(raw)))
}

pub(crate) fn upsert_dsync_links(
    content: &str,
    gdoc: Option<&str>,
    linear: Option<&str>,
    git: Option<&str>,
) -> String {
    let mut lines = Vec::new();
    if let Some(url) = gdoc {
        lines.push(format!("- GDocs: {}", url));
    }
    if let Some(url) = linear {
        lines.push(format!("- Linear: {}", url));
    }
    if let Some(url) = git {
        lines.push(format!("- Git: {}", url));
    }

    if lines.is_empty() {
        return content.to_string();
    }

    let block = format!(
        "{DSYNC_LINKS_START}\n## Document Links\n{}\n{DSYNC_LINKS_END}",
        lines.join("\n")
    );

    if let Some(start) = content.find(DSYNC_LINKS_START)
        && let Some(end_rel) = content[start..].find(DSYNC_LINKS_END)
    {
        let end = start + end_rel + DSYNC_LINKS_END.len();
        let mut output = String::new();
        output.push_str(&content[..start]);
        if !output.ends_with('\n') {
            output.push('\n');
        }
        output.push_str(&block);
        output.push('\n');
        output.push_str(content[end..].trim_start_matches('\n'));
        return output;
    }

    let mut output = content.trim_end().to_string();
    output.push_str("\n\n");
    output.push_str(&block);
    output.push('\n');
    output
}

pub(crate) fn first_heading(markdown: &str) -> Option<String> {
    markdown
        .lines()
        .find_map(|line| line.strip_prefix("# ").map(|text| text.trim().to_string()))
}

pub(crate) fn file_stem_fallback(path: &Path) -> String {
    path.file_stem()
        .and_then(|segment| segment.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| "document".to_string())
}

pub(crate) fn normalize_newlines(input: &str) -> String {
    input.replace("\r\n", "\n")
}

pub(crate) fn slugify(title: &str) -> String {
    let mut output = String::new();
    let mut previous_was_dash = false;

    for ch in title.chars() {
        let normalized = ch.to_ascii_lowercase();
        if normalized.is_ascii_alphanumeric() {
            output.push(normalized);
            previous_was_dash = false;
        } else if !previous_was_dash {
            output.push('-');
            previous_was_dash = true;
        }
    }

    output.trim_matches('-').chars().take(80).collect()
}

#[cfg(test)]
mod tests {
    use super::{Frontmatter, parse_frontmatter, slugify, upsert_dsync_links};

    #[test]
    fn parses_frontmatter_and_preserves_body() {
        let raw = "---\ngdocUrl: https://docs.google.com/document/d/123/edit\n---\n\n# Title\n";

        let (frontmatter, body) = parse_frontmatter(raw).unwrap();

        assert_eq!(
            frontmatter.gdoc_url.as_deref(),
            Some("https://docs.google.com/document/d/123/edit")
        );
        assert_eq!(body, "\n# Title\n");
    }

    #[test]
    fn keeps_content_without_frontmatter() {
        let (frontmatter, body) = parse_frontmatter("# Title\r\nBody").unwrap();

        assert_eq!(frontmatter.gdoc_url, None);
        assert_eq!(body, "# Title\nBody");
    }

    #[test]
    fn upserts_existing_dsync_block() {
        let content = "# Title\n\n<!-- dsync-links:start -->\nold\n<!-- dsync-links:end -->\n";

        let updated = upsert_dsync_links(content, Some("gdoc"), Some("linear"), None);

        assert!(updated.contains("- GDocs: gdoc"));
        assert!(updated.contains("- Linear: linear"));
        assert!(!updated.contains("\nold\n"));
    }

    #[test]
    fn slugify_normalizes_and_limits_length() {
        let title = "Rust & Google Docs / Linear + Markdown ".repeat(4);

        let slug = slugify(&title);

        assert!(slug.starts_with("rust-google-docs-linear-markdown"));
        assert!(slug.len() <= 80);
    }

    #[test]
    fn empty_frontmatter_round_trips_to_default() {
        let (frontmatter, body) = parse_frontmatter("---\n\n---\ncontent").unwrap();

        assert_eq!(frontmatter, Frontmatter::default());
        assert_eq!(body, "content");
    }
}
