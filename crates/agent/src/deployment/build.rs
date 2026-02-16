use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Result};
use sysinfo::System;

use crate::system::PrivilegeWrapper;

const MIN_RAM_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const SWAP_FILE_PATH: &str = "/opt/nanoscale/tmp/nanoscale.swap";
const SOURCE_BASE_PATH: &str = "/opt/nanoscale/sites";

#[derive(Debug)]
pub struct BuildSystem;

impl BuildSystem {
    pub fn execute(
        project_id: &str,
        repo_dir: &Path,
        build_command: &str,
        privilege_wrapper: &PrivilegeWrapper,
    ) -> Result<PathBuf> {
        Self::ensure_swap_if_low_ram(privilege_wrapper)?;
        Self::run_install(repo_dir)?;
        Self::run_build(repo_dir, build_command)?;

        let standalone_dir = repo_dir.join(".next/standalone");
        if !standalone_dir.is_dir() {
            bail!(
                "expected artifact directory not found: {}",
                standalone_dir.display()
            );
        }

        let destination_dir = PathBuf::from(format!("{SOURCE_BASE_PATH}/{project_id}/source"));
        Self::replace_directory(&standalone_dir, &destination_dir)?;
        Self::apply_project_ownership(project_id, &destination_dir, privilege_wrapper)?;

        Ok(destination_dir)
    }

    fn ensure_swap_if_low_ram(privilege_wrapper: &PrivilegeWrapper) -> Result<()> {
        let mut system = System::new_all();
        system.refresh_memory();

        if system.total_memory() >= MIN_RAM_BYTES {
            return Ok(());
        }

        let swap_file_path = Path::new(SWAP_FILE_PATH);
        if swap_file_path.exists() {
            return Ok(());
        }

        privilege_wrapper.run("/usr/bin/fallocate", &["-l", "2G", SWAP_FILE_PATH])?;
        Ok(())
    }

    fn run_install(repo_dir: &Path) -> Result<()> {
        let bun_binary = Self::bun_binary()?;

        let output = Command::new(bun_binary)
            .arg("install")
            .arg("--frozen-lockfile")
            .current_dir(repo_dir)
            .output()
            .map_err(|error| anyhow::anyhow!("failed to execute bun install command: {error}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("bun install failed: {stderr}");
        }

        Ok(())
    }

    fn run_build(repo_dir: &Path, build_command: &str) -> Result<()> {
        if build_command.trim() != "bun run build" {
            bail!("unsupported build command for phase 3.2: {build_command}");
        }

        let bun_binary = Self::bun_binary()?;

        let output = Command::new(bun_binary)
            .arg("run")
            .arg("build")
            .current_dir(repo_dir)
            .output()
            .map_err(|error| anyhow::anyhow!("failed to execute bun run build command: {error}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("bun run build failed: {stderr}");
        }

        Ok(())
    }

    fn replace_directory(source_dir: &Path, destination_dir: &Path) -> Result<()> {
        if destination_dir.exists() {
            fs::remove_dir_all(destination_dir)?;
        }

        if let Some(parent_dir) = destination_dir.parent() {
            fs::create_dir_all(parent_dir)?;
        }

        Self::copy_directory_recursive(source_dir, destination_dir)?;
        Ok(())
    }

    fn copy_directory_recursive(source_dir: &Path, destination_dir: &Path) -> Result<()> {
        fs::create_dir_all(destination_dir)?;

        for entry in fs::read_dir(source_dir)? {
            let entry = entry?;
            let source_path = entry.path();
            let destination_path = destination_dir.join(entry.file_name());

            if source_path.is_dir() {
                Self::copy_directory_recursive(&source_path, &destination_path)?;
            } else {
                fs::copy(&source_path, &destination_path)?;
            }
        }

        Ok(())
    }

    fn apply_project_ownership(
        project_id: &str,
        destination_dir: &Path,
        privilege_wrapper: &PrivilegeWrapper,
    ) -> Result<()> {
        let owner = format!("nanoscale-{project_id}:nanoscale-{project_id}");
        let destination = destination_dir
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("invalid destination path"))?;

        privilege_wrapper.run("/usr/bin/chown", &["-R", &owner, destination])?;
        Ok(())
    }

    fn bun_binary() -> Result<String> {
        if let Ok(configured_binary) = std::env::var("NANOSCALE_BUN_BIN") {
            let trimmed_binary = configured_binary.trim();
            if !trimmed_binary.is_empty() {
                return Ok(trimmed_binary.to_string());
            }
        }

        for candidate in ["/usr/bin/bun", "/bin/bun", "/usr/local/bin/bun"] {
            if Path::new(candidate).is_file() {
                return Ok(candidate.to_string());
            }
        }

        if let Ok(path_value) = std::env::var("PATH") {
            for path_entry in path_value.split(':') {
                if path_entry.is_empty() {
                    continue;
                }

                let candidate_path = Path::new(path_entry).join("bun");
                if candidate_path.is_file() {
                    return Ok(candidate_path.to_string_lossy().to_string());
                }
            }
        }

        let current_path = std::env::var("PATH").unwrap_or_default();
        bail!("bun binary not found; install bun or set NANOSCALE_BUN_BIN (PATH={current_path})")
    }
}
