use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Result};

use crate::deployment::build::AppRuntime;
use crate::system::PrivilegeWrapper;

const TMP_BASE_PATH: &str = "/opt/nanoscale/tmp";
const SYSTEMD_TARGET_PATH: &str = "/etc/systemd/system";

#[derive(Debug)]
pub struct SystemdGenerator;

impl SystemdGenerator {
    pub fn generate_and_install(
        project_id: &str,
        source_dir: &Path,
        runtime: &AppRuntime,
        run_command: &str,
        port: u16,
        privilege_wrapper: &PrivilegeWrapper,
    ) -> Result<()> {
        let service_name = format!("nanoscale-{project_id}");
        let tmp_service_path = PathBuf::from(format!("{TMP_BASE_PATH}/{service_name}.service"));
        let tmp_socket_path = PathBuf::from(format!("{TMP_BASE_PATH}/{service_name}.socket"));

        if let Some(parent_dir) = tmp_service_path.parent() {
            fs::create_dir_all(parent_dir)?;
        }

        let source_dir_string = source_dir
            .to_str()
            .ok_or_else(|| anyhow!("invalid source path"))?;

        let service_template = Self::service_template(
            &service_name,
            project_id,
            source_dir_string,
            runtime,
            run_command,
            port,
        )?;
        let socket_template = Self::socket_template(&service_name, port);

        fs::write(&tmp_service_path, service_template)?;
        fs::write(&tmp_socket_path, socket_template)?;

        let service_target = format!("{SYSTEMD_TARGET_PATH}/{service_name}.service");
        let socket_target = format!("{SYSTEMD_TARGET_PATH}/{service_name}.socket");
        let tmp_service_string = tmp_service_path
            .to_str()
            .ok_or_else(|| anyhow!("invalid temp service path"))?;
        let tmp_socket_string = tmp_socket_path
            .to_str()
            .ok_or_else(|| anyhow!("invalid temp socket path"))?;

        privilege_wrapper.run("/usr/bin/mv", &[tmp_service_string, &service_target])?;
        privilege_wrapper.run("/usr/bin/mv", &[tmp_socket_string, &socket_target])?;
        privilege_wrapper.run("/usr/bin/systemctl", &["daemon-reload"])?;

        Ok(())
    }

    fn service_template(
        service_name: &str,
        project_id: &str,
        source_dir: &str,
        runtime: &AppRuntime,
        run_command: &str,
        port: u16,
    ) -> Result<String> {
        let exec_start = Self::resolve_exec_start(source_dir, runtime, run_command, port)?;

        Ok(format!(
            "[Unit]\nDescription=NanoScale app service ({service_name})\nAfter=network.target\n\n[Service]\nType=simple\nUser=nanoscale-{project_id}\nGroup=nanoscale-{project_id}\nWorkingDirectory={source_dir}\nEnvironment=NODE_ENV=production\nEnvironment=PORT={port}\nExecStart={exec_start}\nRestart=always\nRestartSec=2\n\n# SECURITY HARDENING\nProtectSystem=strict\nProtectHome=yes\nPrivateTmp=yes\nNoNewPrivileges=yes\nProtectProc=invisible\nReadWritePaths={source_dir}\n\n[Install]\nWantedBy=multi-user.target\n"
        ))
    }

    fn resolve_exec_start(
        source_dir: &str,
        runtime: &AppRuntime,
        run_command: &str,
        port: u16,
    ) -> Result<String> {
        let trimmed_run_command = run_command.trim();
        if trimmed_run_command.is_empty() {
            return Ok(match runtime {
                AppRuntime::StandaloneNode => format!("/usr/bin/node {source_dir}/server.js"),
                AppRuntime::BunStart { bun_binary } => {
                    format!("{bun_binary} run start -- --hostname 127.0.0.1 --port {port}")
                }
            });
        }

        let (program, arguments) = Self::parse_command(trimmed_run_command)?;
        let executable = if program == "bun" {
            Self::bun_binary()?
        } else {
            program
        };

        if arguments.is_empty() {
            return Ok(executable);
        }

        Ok(format!("{executable} {}", arguments.join(" ")))
    }

    fn parse_command(raw_command: &str) -> Result<(String, Vec<String>)> {
        let trimmed_command = raw_command.trim();
        if trimmed_command.is_empty() {
            bail!("run command cannot be empty");
        }

        let forbidden_characters = [';', '|', '&', '>', '<', '`', '$', '\n', '\r'];
        if trimmed_command
            .chars()
            .any(|character| forbidden_characters.contains(&character))
        {
            bail!("run command contains unsupported shell control characters");
        }

        let mut parts = trimmed_command.split_whitespace();
        let program = parts
            .next()
            .ok_or_else(|| anyhow!("run command program is missing"))?
            .to_string();
        let arguments = parts.map(ToString::to_string).collect::<Vec<String>>();

        Ok((program, arguments))
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

    fn socket_template(service_name: &str, port: u16) -> String {
        format!(
            "[Unit]\nDescription=NanoScale app socket ({service_name})\nPartOf={service_name}.service\n\n[Socket]\nListenStream=127.0.0.1:{port}\nNoDelay=true\n\n[Install]\nWantedBy=sockets.target\n"
        )
    }
}
