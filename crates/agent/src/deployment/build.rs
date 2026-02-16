use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Result};
use sysinfo::System;

use crate::system::PrivilegeWrapper;

const MIN_RAM_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const SWAP_FILE_PATH: &str = "/opt/nanoscale/tmp/nanoscale.swap";
const SOURCE_BASE_PATH: &str = "/opt/nanoscale/sites";
const RUNTIME_PATH: &str = "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin";

#[derive(Debug)]
pub struct BuildSystem;

#[derive(Clone, Debug)]
pub struct BuildSettings {
    pub build_command: String,
    pub output_directory: String,
    pub install_command: String,
}

#[derive(Clone, Debug)]
pub enum AppRuntime {
    StandaloneNode,
    BunStart { bun_binary: String },
}

#[derive(Debug)]
pub struct BuildOutput {
    pub source_dir: PathBuf,
    pub runtime: AppRuntime,
}

impl BuildSystem {
    pub fn execute(
        project_id: &str,
        repo_dir: &Path,
        settings: &BuildSettings,
        privilege_wrapper: &PrivilegeWrapper,
    ) -> Result<BuildOutput> {
        Self::ensure_swap_if_low_ram(privilege_wrapper)
            .map_err(|error| anyhow::anyhow!("swap provisioning failed: {error:#}"))?;
        Self::run_command(repo_dir, &settings.install_command, "dependency install")
            .map_err(|error| anyhow::anyhow!("dependency install failed: {error:#}"))?;
        Self::run_command(repo_dir, &settings.build_command, "application build")
            .map_err(|error| anyhow::anyhow!("application build failed: {error:#}"))?;

        let destination_dir = PathBuf::from(format!("{SOURCE_BASE_PATH}/{project_id}/source"));
        let artifact_source_dir =
            Self::resolve_output_directory(repo_dir, &settings.output_directory)?;

        Self::replace_directory(&artifact_source_dir, &destination_dir)
            .map_err(|error| anyhow::anyhow!("artifact copy failed: {error:#}"))?;

        let runtime = if destination_dir.join("server.js").is_file() {
            AppRuntime::StandaloneNode
        } else {
            let bun_binary = Self::bun_binary()
                .map_err(|error| anyhow::anyhow!("bun runtime resolution failed: {error:#}"))?;
            AppRuntime::BunStart { bun_binary }
        };

        Self::apply_project_ownership(project_id, &destination_dir, privilege_wrapper)
            .map_err(|error| anyhow::anyhow!("artifact ownership setup failed: {error:#}"))?;

        Ok(BuildOutput {
            source_dir: destination_dir,
            runtime,
        })
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

    fn run_command(repo_dir: &Path, raw_command: &str, command_label: &str) -> Result<()> {
        let (program, arguments) = Self::parse_command(raw_command)?;

        let executable = if program == "bun" {
            Self::bun_binary()?
        } else {
            program
        };

        let mut command = Command::new(executable);
        Self::apply_runtime_env(&mut command);

        let output = command
            .args(arguments)
            .current_dir(repo_dir)
            .output()
            .map_err(|error| {
                anyhow::anyhow!("failed to execute {command_label} command: {error}")
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("{command_label} command failed: {stderr}");
        }

        Ok(())
    }

    fn parse_command(raw_command: &str) -> Result<(String, Vec<String>)> {
        let trimmed_command = raw_command.trim();
        if trimmed_command.is_empty() {
            bail!("command cannot be empty");
        }

        let forbidden_characters = [';', '|', '&', '>', '<', '`', '$', '\n', '\r'];
        if trimmed_command
            .chars()
            .any(|character| forbidden_characters.contains(&character))
        {
            bail!("command contains unsupported shell control characters");
        }

        let mut parts = trimmed_command.split_whitespace();
        let program = parts
            .next()
            .ok_or_else(|| anyhow::anyhow!("command program is missing"))?
            .to_string();
        let arguments = parts.map(ToString::to_string).collect::<Vec<String>>();

        Ok((program, arguments))
    }

    fn resolve_output_directory(repo_dir: &Path, output_directory: &str) -> Result<PathBuf> {
        let trimmed_output_directory = output_directory.trim();
        if trimmed_output_directory.is_empty() {
            return Ok(repo_dir.to_path_buf());
        }

        let candidate_dir = repo_dir.join(trimmed_output_directory);
        if candidate_dir.is_dir() {
            return Ok(candidate_dir);
        }

        let dot_next_dir = repo_dir.join(".next");
        if trimmed_output_directory == ".next/standalone" && dot_next_dir.is_dir() {
            return Ok(repo_dir.to_path_buf());
        }

        bail!(
            "configured output directory not found: {}",
            candidate_dir.display()
        )
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

    fn apply_runtime_env(command: &mut Command) {
        command.env("PATH", RUNTIME_PATH);
    }
}
