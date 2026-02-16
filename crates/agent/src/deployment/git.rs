use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, bail, Result};
use regex::Regex;

#[derive(Debug)]
pub struct Git;

impl Git {
    pub fn clone(repo_url: &str, target_dir: &Path) -> Result<()> {
        Self::validate_repo_url(repo_url)?;
        let git_binary = Self::git_binary()?;

        let output = Command::new(git_binary)
            .arg("clone")
            .arg("--depth")
            .arg("1")
            .arg(repo_url)
            .arg(target_dir)
            .output()
            .map_err(|error| anyhow!("failed to execute git clone command: {error}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("git clone failed: {stderr}");
        }

        Ok(())
    }

    pub fn checkout(repo_dir: &Path, branch: &str) -> Result<()> {
        Self::validate_branch(branch)?;
        let git_binary = Self::git_binary()?;

        let output = Command::new(git_binary)
            .arg("-C")
            .arg(repo_dir)
            .arg("checkout")
            .arg(branch)
            .output()
            .map_err(|error| anyhow!("failed to execute git checkout command: {error}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("git checkout failed: {stderr}");
        }

        Ok(())
    }

    pub fn validate_repo_url(repo_url: &str) -> Result<()> {
        let regex = Regex::new(r"^https://[A-Za-z0-9._~:/?#\[\]@!$&'()*+,;=%-]+$")
            .map_err(|error| anyhow!("invalid repo url validator: {error}"))?;

        if !regex.is_match(repo_url) {
            bail!("repo URL must be HTTPS and match allowlisted characters");
        }

        Ok(())
    }

    pub fn validate_branch(branch: &str) -> Result<()> {
        let regex = Regex::new(r"^[a-zA-Z0-9-_]+$")
            .map_err(|error| anyhow!("invalid branch validator: {error}"))?;

        if !regex.is_match(branch) {
            bail!("branch must match ^[a-zA-Z0-9-_]+$");
        }

        Ok(())
    }

    fn git_binary() -> Result<String> {
        if let Ok(configured_binary) = std::env::var("NANOSCALE_GIT_BIN") {
            let trimmed_binary = configured_binary.trim();
            if !trimmed_binary.is_empty() {
                return Ok(trimmed_binary.to_string());
            }
        }

        for candidate in ["/usr/bin/git", "/bin/git", "/usr/local/bin/git"] {
            if Path::new(candidate).is_file() {
                return Ok(candidate.to_string());
            }
        }

        if let Ok(path_value) = std::env::var("PATH") {
            for path_entry in path_value.split(':') {
                if path_entry.is_empty() {
                    continue;
                }

                let candidate_path = Path::new(path_entry).join("git");
                if candidate_path.is_file() {
                    return Ok(candidate_path.to_string_lossy().to_string());
                }
            }
        }

        let current_path = std::env::var("PATH").unwrap_or_default();
        bail!("git binary not found; install git or set NANOSCALE_GIT_BIN (PATH={current_path})")
    }
}
