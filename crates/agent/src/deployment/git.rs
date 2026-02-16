use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, bail, Result};
use regex::Regex;

#[derive(Debug)]
pub struct Git;

impl Git {
    pub fn clone(repo_url: &str, target_dir: &Path) -> Result<()> {
        Self::validate_repo_url(repo_url)?;

        let output = Command::new("git")
            .arg("clone")
            .arg("--depth")
            .arg("1")
            .arg(repo_url)
            .arg(target_dir)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("git clone failed: {stderr}");
        }

        Ok(())
    }

    pub fn checkout(repo_dir: &Path, branch: &str) -> Result<()> {
        Self::validate_branch(branch)?;

        let output = Command::new("git")
            .arg("-C")
            .arg(repo_dir)
            .arg("checkout")
            .arg(branch)
            .output()?;

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
}
