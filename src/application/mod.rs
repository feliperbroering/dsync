use std::path::PathBuf;

use anyhow::{Result, anyhow};

use crate::cli::{Cli, Command};
use crate::domain::document::{Frontmatter, MdDoc};
use crate::domain::markdown::{
    file_stem_fallback, first_heading, normalize_newlines, read_md, slugify, upsert_dsync_links,
    write_md,
};
use crate::integrations::git::git_blob_url_for_path;
use crate::providers::{google_docs, linear};
use crate::ui::{pick_linear_destination, prompt_drive_folder_id};

pub fn run() -> Result<()> {
    let command = Cli::parse_args().into_command()?;

    match command {
        Command::ImportFromGdoc { doc_id } => import_from_gdoc(&doc_id),
        Command::ImportFromLinear { doc_id } => import_from_linear(&doc_id),
        Command::Sync {
            path,
            with_gdoc,
            with_linear,
        } => sync_markdown(path, with_gdoc, with_linear),
    }
}

fn import_from_gdoc(gdoc_id: &str) -> Result<()> {
    let gdoc = google_docs::get_document(gdoc_id)?;
    let path: PathBuf = format!("{}.md", slugify(&gdoc.title)).into();
    let mut frontmatter = Frontmatter {
        gdoc_url: Some(gdoc.url.clone()),
        ..Frontmatter::default()
    };

    let mut content = ensure_trailing_newline(&gdoc.text);
    let git_url = git_blob_url_for_path(&path).ok();
    frontmatter.git_url = git_url.clone();
    content = upsert_dsync_links(
        &content,
        frontmatter.gdoc_url.as_deref(),
        None,
        git_url.as_deref(),
    );

    let document = MdDoc {
        path: path.clone(),
        frontmatter,
        content,
    };
    write_md(&document)?;
    println!("✅ Imported from Google Docs to {}", path.display());
    Ok(())
}

fn import_from_linear(linear_id: &str) -> Result<()> {
    let linear_doc = linear::get_document(linear_id)?;
    let path: PathBuf = format!("{}.md", slugify(&linear_doc.title)).into();
    let mut frontmatter = Frontmatter {
        linear_doc_id: Some(linear_doc.id.clone()),
        linear_doc_url: Some(linear_doc.url.clone()),
        ..Frontmatter::default()
    };

    let mut content = ensure_trailing_newline(&linear_doc.content);
    let git_url = git_blob_url_for_path(&path).ok();
    frontmatter.git_url = git_url.clone();
    content = upsert_dsync_links(
        &content,
        None,
        frontmatter.linear_doc_url.as_deref(),
        git_url.as_deref(),
    );

    let document = MdDoc {
        path: path.clone(),
        frontmatter,
        content,
    };
    write_md(&document)?;
    println!("✅ Imported from Linear to {}", path.display());
    Ok(())
}

fn sync_markdown(path: std::path::PathBuf, want_gdoc: bool, want_linear: bool) -> Result<()> {
    let mut document = read_md(&path)?;
    let title = first_heading(&document.content).unwrap_or_else(|| file_stem_fallback(&path));

    if let Ok(git_url) = git_blob_url_for_path(&path) {
        document.frontmatter.git_url = Some(git_url);
    }

    let should_sync_gdoc = want_gdoc || document.frontmatter.gdoc_url.is_some();
    let should_sync_linear = want_linear
        || document.frontmatter.linear_doc_id.is_some()
        || document.frontmatter.linear_doc_url.is_some();

    maybe_create_gdoc(&mut document, &title, should_sync_gdoc)?;
    maybe_create_linear_doc(&mut document, &title, should_sync_linear)?;

    let sync_content = upsert_links(&document);
    document.content = sync_content.clone();
    write_md(&document)?;

    if let Some(gdoc_url) = &document.frontmatter.gdoc_url {
        let gdoc_id = google_docs::doc_id_from_url(gdoc_url)
            .ok_or_else(|| anyhow!("Invalid gdocUrl in frontmatter"))?;
        google_docs::write_document(&gdoc_id, &sync_content)?;
        println!("✅ Google Doc updated.");
    }

    if let Some(linear_id) = linear::doc_id_from_frontmatter(&document.frontmatter) {
        linear::update_document(&linear_id, &sync_content)?;
        println!("✅ Linear Doc updated.");
    }

    write_md(&document)?;
    println!("🚀 Tri-sync completed: {}", document.path.display());
    Ok(())
}

fn maybe_create_gdoc(document: &mut MdDoc, title: &str, should_sync: bool) -> Result<()> {
    if !should_sync || document.frontmatter.gdoc_url.is_some() {
        return Ok(());
    }

    let folder_id = prompt_drive_folder_id()?;
    let created = google_docs::create_document(title, folder_id.as_deref())?;
    document.frontmatter.gdoc_url = Some(created.url);
    println!("✨ Google Doc created.");
    Ok(())
}

fn maybe_create_linear_doc(document: &mut MdDoc, title: &str, should_sync: bool) -> Result<()> {
    if !should_sync || document.frontmatter.linear_doc_id.is_some() {
        return Ok(());
    }

    let destination = pick_linear_destination()?;
    let created = linear::create_document(
        title,
        &document.content,
        &destination.team_id,
        destination.project_id.as_deref(),
    )?;

    document.frontmatter.linear_doc_id = Some(created.id.clone());
    document.frontmatter.linear_doc_url = Some(created.url.clone());
    println!("✨ Linear Doc created.");
    Ok(())
}

fn upsert_links(document: &MdDoc) -> String {
    upsert_dsync_links(
        &document.content,
        document.frontmatter.gdoc_url.as_deref(),
        document.frontmatter.linear_doc_url.as_deref(),
        document.frontmatter.git_url.as_deref(),
    )
}

fn ensure_trailing_newline(content: &str) -> String {
    let mut normalized = normalize_newlines(content);
    if !normalized.ends_with('\n') {
        normalized.push('\n');
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::ensure_trailing_newline;

    #[test]
    fn appends_newline_only_when_missing() {
        assert_eq!(ensure_trailing_newline("abc"), "abc\n");
        assert_eq!(ensure_trailing_newline("abc\n"), "abc\n");
    }
}
