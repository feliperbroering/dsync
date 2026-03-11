use std::path::PathBuf;

use anyhow::{Result, anyhow};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "dsync",
    version,
    about = "Sync Markdown with GDocs and Linear Docs"
)]
pub(crate) struct Cli {
    /// Markdown file path to sync (ex: ~/docs/doc.md)
    file: Option<String>,

    /// Google Docs mode:
    /// - --gdoc              => sync/create using frontmatter
    /// - --gdoc <DOC_ID>     => import doc into current folder
    #[arg(long, num_args = 0..=1, value_name = "DOC_ID")]
    gdoc: Option<Option<String>>,

    /// Linear Docs mode:
    /// - --linear            => sync/create using frontmatter
    /// - --linear <DOC_ID>   => import doc into current folder
    #[arg(long, num_args = 0..=1, value_name = "DOC_ID")]
    linear: Option<Option<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Command {
    ImportFromGdoc {
        doc_id: String,
    },
    ImportFromLinear {
        doc_id: String,
    },
    Sync {
        path: PathBuf,
        with_gdoc: bool,
        with_linear: bool,
    },
}

impl Cli {
    pub(crate) fn parse_args() -> Self {
        Self::parse()
    }

    pub(crate) fn into_command(self) -> Result<Command> {
        if let Some(Some(doc_id)) = self.gdoc.clone()
            && self.file.is_none()
            && self.linear.is_none()
        {
            return Ok(Command::ImportFromGdoc { doc_id });
        }

        if let Some(Some(doc_id)) = self.linear.clone()
            && self.file.is_none()
            && self.gdoc.is_none()
        {
            return Ok(Command::ImportFromLinear { doc_id });
        }

        let file = self
            .file
            .ok_or_else(|| anyhow!("Provide a .md file or use --gdoc <id> / --linear <id>"))?;

        Ok(Command::Sync {
            path: expand_tilde(&file),
            with_gdoc: self.gdoc.is_some(),
            with_linear: self.linear.is_some(),
        })
    }
}

fn expand_tilde(path: &str) -> PathBuf {
    if !path.starts_with('~') {
        return PathBuf::from(path);
    }

    let home = std::env::var("HOME").unwrap_or_default();
    let rest = path
        .strip_prefix("~/")
        .unwrap_or(path.trim_start_matches('~'));

    PathBuf::from(home).join(rest)
}

#[cfg(test)]
mod tests {
    use super::{Cli, Command};

    #[test]
    fn resolves_import_from_gdoc_when_only_doc_id_is_informed() {
        let cli = Cli {
            file: None,
            gdoc: Some(Some("abc123".to_string())),
            linear: None,
        };

        let command = cli.into_command().unwrap();

        assert_eq!(
            command,
            Command::ImportFromGdoc {
                doc_id: "abc123".to_string()
            }
        );
    }

    #[test]
    fn resolves_sync_when_file_is_present() {
        let cli = Cli {
            file: Some("docs/note.md".to_string()),
            gdoc: Some(None),
            linear: None,
        };

        let command = cli.into_command().unwrap();

        assert_eq!(
            command,
            Command::Sync {
                path: "docs/note.md".into(),
                with_gdoc: true,
                with_linear: false,
            }
        );
    }
}
