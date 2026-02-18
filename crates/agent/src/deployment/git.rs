use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, bail, Result};
use regex::Regex;

#[derive(Debug)]
pub struct Git;

impl Git {
    /// Clones an HTTPS repository into `target_dir`.
    ///
    /// # Errors
    /// Returns an error if the repo URL is invalid, git cannot be located, the clone command
    /// cannot be executed, or git exits unsuccessfully.
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

    /// Checks out `branch` in `repo_dir`.
    ///
    /// # Errors
    /// Returns an error if the branch is invalid, git cannot be located, the checkout command
    /// cannot be executed, or git exits unsuccessfully.
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

    /// Validates that `repo_url` is HTTPS and contains only allowlisted characters.
    ///
    /// # Errors
    /// Returns an error if the validator regex cannot be compiled or the URL does not pass.
    pub fn validate_repo_url(repo_url: &str) -> Result<()> {
        let regex = Regex::new(r"^https://[A-Za-z0-9._~:/?#\[\]@!$&'()*+,;=%-]+$")
            .map_err(|error| anyhow!("invalid repo url validator: {error}"))?;

        if !regex.is_match(repo_url) {
            bail!("repo URL must be HTTPS and match allowlisted characters");
        }

        Ok(())
    }

    /// Validates that `branch` matches an allowlisted format.
    ///
    /// # Errors
    /// Returns an error if the validator regex cannot be compiled or the branch does not pass.
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn validate_repo_url_accepts_https_and_rejects_other_schemes() {
        Git::validate_repo_url("https://example.com/repo.git").expect("https should pass");
        assert!(Git::validate_repo_url("git@example.com:repo.git").is_err());
        assert!(Git::validate_repo_url("http://example.com/repo.git").is_err());
    }

    #[test]
    fn validate_branch_allows_simple_names() {
        Git::validate_branch("main").expect("branch should pass");
        Git::validate_branch("feature-1").expect("branch should pass");
        assert!(Git::validate_branch("feature/test").is_err());
        assert!(Git::validate_branch("feature;rm -rf /").is_err());
    }

    #[test]
    fn git_binary_prefers_env_override_even_if_nonexistent() {
        let _guard = env_lock().lock().expect("env lock poisoned");
        std::env::set_var("NANOSCALE_GIT_BIN", "  /custom/git  ");
        let resolved = Git::git_binary().expect("git binary resolution");
        assert_eq!(resolved, "/custom/git");
        std::env::remove_var("NANOSCALE_GIT_BIN");
    }
}
