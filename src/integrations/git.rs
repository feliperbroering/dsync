use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, anyhow, bail};

pub(crate) fn git_blob_url_for_path(path: &Path) -> Result<String> {
    let absolute_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let start_dir = if absolute_path.is_file() {
        absolute_path
            .parent()
            .unwrap_or(Path::new("."))
            .to_path_buf()
    } else {
        absolute_path.clone()
    };

    let repo_root = run_cmd(&["git", "rev-parse", "--show-toplevel"], Some(&start_dir))?;
    let repo_root_path = canonicalize_or_fallback(PathBuf::from(&repo_root));
    let remote = run_cmd(
        &["git", "remote", "get-url", "origin"],
        Some(&repo_root_path),
    )?;
    let branch = run_cmd(
        &["git", "rev-parse", "--abbrev-ref", "HEAD"],
        Some(&repo_root_path),
    )?;

    let relative_path = absolute_path
        .strip_prefix(&repo_root_path)
        .ok()
        .and_then(|path| path.to_str())
        .ok_or_else(|| anyhow!("Failed to calculate relative git path"))?
        .replace('\\', "/");

    let remote_url = parse_github_remote_to_https(&remote)?;
    Ok(format!("{remote_url}/blob/{branch}/{relative_path}"))
}

fn canonicalize_or_fallback(path: PathBuf) -> PathBuf {
    path.canonicalize().unwrap_or(path)
}

fn run_cmd(args: &[&str], cwd: Option<&Path>) -> Result<String> {
    let mut command = Command::new(args[0]);
    for arg in &args[1..] {
        command.arg(arg);
    }

    if let Some(dir) = cwd {
        command.current_dir(dir);
    }

    let output = command
        .output()
        .with_context(|| format!("Failed to execute {:?}", args))?;

    if !output.status.success() {
        bail!(
            "Command {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn parse_github_remote_to_https(remote: &str) -> Result<String> {
    if let Some(stripped) = remote.strip_prefix("git@github.com:") {
        return Ok(format!(
            "https://github.com/{}",
            stripped.trim_end_matches(".git")
        ));
    }

    if let Some(stripped) = remote.strip_prefix("https://github.com/") {
        return Ok(format!(
            "https://github.com/{}",
            stripped.trim_end_matches(".git")
        ));
    }

    bail!("Unsupported git remote for generating gitUrl: {}", remote)
}

#[cfg(test)]
mod tests {
    use super::parse_github_remote_to_https;

    #[test]
    fn converts_ssh_remote_to_https() {
        let remote = parse_github_remote_to_https("git@github.com:owner/repo.git").unwrap();
        assert_eq!(remote, "https://github.com/owner/repo");
    }

    #[test]
    fn converts_https_remote_to_https_without_git_suffix() {
        let remote = parse_github_remote_to_https("https://github.com/owner/repo.git").unwrap();
        assert_eq!(remote, "https://github.com/owner/repo");
    }
}
