use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Result};
use sysinfo::System;

use crate::system::PrivilegeWrapper;

mod bun;

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
    /// Executes the build pipeline and installs build artifacts into the project sites directory.
    ///
    /// # Errors
    /// Returns an error if swap provisioning fails, build commands fail, build artifacts cannot be
    /// copied into place, permissions/ownership cannot be applied, or runtime detection fails.
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

        Self::replace_directory(&artifact_source_dir, &destination_dir, privilege_wrapper)
            .map_err(|error| anyhow::anyhow!("artifact copy failed: {error:#}"))?;

        Self::ensure_sites_directory_traversable().map_err(|error| {
            anyhow::anyhow!("sites directory permission setup failed: {error:#}")
        })?;

        let runtime = if destination_dir.join("server.js").is_file()
            || destination_dir.join(".next/standalone/server.js").is_file()
        {
            AppRuntime::StandaloneNode
        } else {
            let bun_binary = bun::bun_binary()
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

    fn ensure_sites_directory_traversable() -> Result<()> {
        let sites_dir = Path::new(SOURCE_BASE_PATH);
        if !sites_dir.exists() {
            return Ok(());
        }

        let metadata = fs::metadata(sites_dir)?;
        let current_mode = metadata.permissions().mode();
        let target_mode = current_mode | 0o111;

        if target_mode != current_mode {
            let mut permissions = metadata.permissions();
            permissions.set_mode(target_mode);
            fs::set_permissions(sites_dir, permissions)?;
        }

        Ok(())
    }

    fn run_command(repo_dir: &Path, raw_command: &str, command_label: &str) -> Result<()> {
        let (program, arguments) = Self::parse_command(raw_command)?;

        let executable = if program == "bun" {
            bun::bun_binary()?
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

    fn replace_directory(
        source_dir: &Path,
        destination_dir: &Path,
        privilege_wrapper: &PrivilegeWrapper,
    ) -> Result<()> {
        if destination_dir.exists() {
            match fs::remove_dir_all(destination_dir) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                    let destination = destination_dir
                        .to_str()
                        .ok_or_else(|| anyhow::anyhow!("invalid destination path"))?;
                    privilege_wrapper.run("/usr/bin/rm", &["-rf", destination])?;
                }
                Err(error) => return Err(error.into()),
            }
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
            let source_metadata = fs::symlink_metadata(&source_path)?;
            let source_type = source_metadata.file_type();

            if source_type.is_symlink() {
                let symlink_target = fs::read_link(&source_path)?;
                symlink(&symlink_target, &destination_path)?;
            } else if source_type.is_dir() {
                Self::copy_directory_recursive(&source_path, &destination_path)?;
            } else if source_type.is_file() {
                fs::copy(&source_path, &destination_path)?;
            } else {
                bail!(
                    "unsupported file type while copying artifacts: {}",
                    source_path.display()
                );
            }
        }

        Ok(())
    }

    fn apply_project_ownership(
        project_id: &str,
        destination_dir: &Path,
        privilege_wrapper: &PrivilegeWrapper,
    ) -> Result<()> {
        Self::ensure_project_system_user(project_id, privilege_wrapper)?;

        let username = format!("nanoscale-{project_id}");
        let primary_group = Command::new("/usr/bin/id")
            .arg("-gn")
            .arg(&username)
            .output()
            .map_err(|error| anyhow::anyhow!("failed to resolve primary group: {error}"))?;

        if !primary_group.status.success() {
            let stderr = String::from_utf8_lossy(&primary_group.stderr);
            bail!("failed to resolve primary group for {username}: {stderr}");
        }

        let group = String::from_utf8_lossy(&primary_group.stdout)
            .trim()
            .to_string();
        let owner = format!("{username}:{group}");
        let destination = destination_dir
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("invalid destination path"))?;

        privilege_wrapper.run("/usr/bin/chown", &["-R", &owner, destination])?;
        Ok(())
    }

    fn ensure_project_system_user(
        project_id: &str,
        privilege_wrapper: &PrivilegeWrapper,
    ) -> Result<()> {
        let username = format!("nanoscale-{project_id}");

        let user_exists = Command::new("/usr/bin/id")
            .arg("-u")
            .arg(&username)
            .status()
            .map(|status| status.success())
            .unwrap_or(false);

        if user_exists {
            return Ok(());
        }

        privilege_wrapper.run("/usr/sbin/useradd", &["-r", "-s", "/bin/false", &username])?;

        Ok(())
    }

    fn apply_runtime_env(command: &mut Command) {
        command.env("PATH", RUNTIME_PATH);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_command_splits_program_and_args() {
        let (program, args) = BuildSystem::parse_command("bun run build").expect("parse");
        assert_eq!(program, "bun");
        assert_eq!(args, vec!["run", "build"]);
    }

    #[test]
    fn parse_command_rejects_shell_control_characters() {
        assert!(BuildSystem::parse_command("echo hi; rm -rf /").is_err());
        assert!(BuildSystem::parse_command("echo hi | cat").is_err());
        assert!(BuildSystem::parse_command("").is_err());
    }

    #[test]
    fn resolve_output_directory_returns_repo_when_empty() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let repo = tempdir.path();
        let resolved = BuildSystem::resolve_output_directory(repo, "").expect("resolve");
        assert_eq!(resolved, repo);
    }

    #[test]
    fn resolve_output_directory_accepts_existing_subdir() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let repo = tempdir.path();
        let out = repo.join("dist");
        std::fs::create_dir_all(&out).expect("mkdir");
        let resolved = BuildSystem::resolve_output_directory(repo, "dist").expect("resolve");
        assert_eq!(resolved, out);
    }

    #[test]
    fn resolve_output_directory_rejects_missing() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let repo = tempdir.path();
        assert!(BuildSystem::resolve_output_directory(repo, "nope").is_err());
    }

    #[test]
    fn replace_directory_copies_files_and_symlinks() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let source = tempdir.path().join("source");
        let dest = tempdir.path().join("dest");
        let privilege_wrapper = PrivilegeWrapper::new();
        std::fs::create_dir_all(&source).expect("mkdir source");
        std::fs::write(source.join("a.txt"), "hello").expect("write");

        let nested = source.join("nested");
        std::fs::create_dir_all(&nested).expect("mkdir nested");
        std::fs::write(nested.join("b.txt"), "world").expect("write");

        let link_target = source.join("a.txt");
        let link_path = source.join("a-link");
        std::os::unix::fs::symlink(&link_target, &link_path).expect("symlink");

        BuildSystem::replace_directory(&source, &dest, &privilege_wrapper).expect("replace");
        assert_eq!(
            std::fs::read_to_string(dest.join("a.txt")).expect("read a.txt"),
            "hello"
        );
        assert_eq!(
            std::fs::read_to_string(dest.join("nested/b.txt")).expect("read nested/b.txt"),
            "world"
        );

        let symlink_meta = std::fs::symlink_metadata(dest.join("a-link")).expect("meta");
        assert!(symlink_meta.file_type().is_symlink());
    }
}
