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
    /// Generates and installs systemd service/socket units for a project and enables the service.
    ///
    /// # Errors
    /// Returns an error if unit files cannot be generated or written, paths are invalid, or
    /// privileged install/enable commands fail.
    pub fn generate_and_install(
        project_id: &str,
        source_dir: &Path,
        runtime: &AppRuntime,
        run_command: &str,
        port: u16,
        privilege_wrapper: &PrivilegeWrapper,
    ) -> Result<()> {
        let service_name = format!("nanoscale-{project_id}");

        let backend_port = backend_port(port)?;
        let socket_proxyd_bin = socket_proxyd_binary()?;

        let tmp_service_path = PathBuf::from(format!("{TMP_BASE_PATH}/{service_name}.service"));
        let tmp_socket_path = PathBuf::from(format!("{TMP_BASE_PATH}/{service_name}.socket"));
        let tmp_proxy_path = PathBuf::from(format!("{TMP_BASE_PATH}/{service_name}-proxy.service"));

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
            backend_port,
        )?;
        let socket_template = Self::socket_template(&service_name, port);
        let proxy_template =
            Self::proxy_service_template(&service_name, backend_port, &socket_proxyd_bin);

        fs::write(&tmp_service_path, service_template)?;
        fs::write(&tmp_socket_path, socket_template)?;
        fs::write(&tmp_proxy_path, proxy_template)?;

        let service_target = format!("{SYSTEMD_TARGET_PATH}/{service_name}.service");
        let socket_target = format!("{SYSTEMD_TARGET_PATH}/{service_name}.socket");
        let proxy_target = format!("{SYSTEMD_TARGET_PATH}/{service_name}-proxy.service");
        let tmp_service_string = tmp_service_path
            .to_str()
            .ok_or_else(|| anyhow!("invalid temp service path"))?;
        let tmp_socket_string = tmp_socket_path
            .to_str()
            .ok_or_else(|| anyhow!("invalid temp socket path"))?;
        let tmp_proxy_string = tmp_proxy_path
            .to_str()
            .ok_or_else(|| anyhow!("invalid temp proxy path"))?;

        privilege_wrapper.run("/usr/bin/mv", &[tmp_service_string, &service_target])?;
        privilege_wrapper.run("/usr/bin/mv", &[tmp_socket_string, &socket_target])?;
        privilege_wrapper.run("/usr/bin/mv", &[tmp_proxy_string, &proxy_target])?;

        privilege_wrapper.run("/usr/bin/chown", &["root:root", &service_target])?;
        privilege_wrapper.run("/usr/bin/chown", &["root:root", &socket_target])?;
        privilege_wrapper.run("/usr/bin/chown", &["root:root", &proxy_target])?;

        privilege_wrapper.run("/usr/bin/systemctl", &["daemon-reload"])?;
        privilege_wrapper.run(
            "/usr/bin/systemctl",
            &["enable", "--now", &format!("{service_name}.service")],
        )?;

        privilege_wrapper.run(
            "/usr/bin/systemctl",
            &["enable", "--now", &format!("{service_name}.socket")],
        )?;

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
            "[Unit]\nDescription=NanoScale app service ({service_name})\nAfter=network.target\n\n[Service]\nType=simple\nUser=nanoscale-{project_id}\nGroup=nanoscale-{project_id}\nWorkingDirectory={source_dir}\nEnvironment=NODE_ENV=production\nEnvironment=PORT={port}\nExecStart={exec_start}\nRestart=always\nRestartSec=2\n\n# ACCOUNTING (for stats)\nCPUAccounting=yes\nMemoryAccounting=yes\nIPAccounting=yes\n\n# SECURITY HARDENING\nProtectSystem=strict\nProtectHome=yes\nPrivateTmp=yes\nNoNewPrivileges=yes\nProtectProc=invisible\nReadWritePaths={source_dir}\n\n[Install]\nWantedBy=multi-user.target\n"
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
            "[Unit]\nDescription=NanoScale app socket ({service_name})\n\n[Socket]\nListenStream=127.0.0.1:{port}\nNoDelay=true\nService={service_name}-proxy.service\n\n[Install]\nWantedBy=sockets.target\n"
        )
    }

    fn proxy_service_template(
        service_name: &str,
        backend_port: u16,
        socket_proxyd_bin: &str,
    ) -> String {
        format!(
            "[Unit]\nDescription=NanoScale app socket proxy ({service_name})\nAfter=network.target\n\n[Service]\nType=simple\nRestart=always\nRestartSec=1\nExecStartPre=/usr/bin/systemctl start {service_name}.service\nExecStart={socket_proxyd_bin} 127.0.0.1:{backend_port}\n"
        )
    }
}

fn socket_proxyd_binary() -> Result<String> {
    if let Ok(configured_binary) = std::env::var("NANOSCALE_SOCKET_PROXYD_BIN") {
        let trimmed = configured_binary.trim();
        if !trimmed.is_empty() {
            if Path::new(trimmed).is_file() {
                return Ok(trimmed.to_string());
            }
            bail!("NANOSCALE_SOCKET_PROXYD_BIN is set but does not exist: {trimmed}");
        }
    }

    for candidate in [
        "/lib/systemd/systemd-socket-proxyd",
        "/usr/lib/systemd/systemd-socket-proxyd",
        "/usr/libexec/systemd/systemd-socket-proxyd",
    ] {
        if Path::new(candidate).is_file() {
            return Ok(candidate.to_string());
        }
    }

    bail!(
        "systemd-socket-proxyd not found; install systemd package providing it or set NANOSCALE_SOCKET_PROXYD_BIN"
    )
}

fn backend_port(front_port: u16) -> Result<u16> {
    let candidate = u32::from(front_port) + 10_000;
    if candidate > u32::from(u16::MAX) {
        bail!("cannot derive backend port from {front_port}; {candidate} exceeds 65535");
    }

    u16::try_from(candidate)
        .map_err(|error| anyhow!("cannot derive backend port from {front_port}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deployment::build::AppRuntime;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn parse_command_rejects_shell_control_characters() {
        assert!(SystemdGenerator::parse_command("echo hi; rm -rf /").is_err());
        assert!(SystemdGenerator::parse_command(" ").is_err());
    }

    #[test]
    fn resolve_exec_start_defaults_based_on_runtime() {
        let standalone = SystemdGenerator::resolve_exec_start(
            "/opt/nanoscale/sites/p1/source",
            &AppRuntime::StandaloneNode,
            "",
            3100,
        )
        .expect("exec");
        assert_eq!(
            standalone,
            "/usr/bin/node /opt/nanoscale/sites/p1/source/server.js"
        );

        let bun = SystemdGenerator::resolve_exec_start(
            "/opt/nanoscale/sites/p1/source",
            &AppRuntime::BunStart {
                bun_binary: "/custom/bun".to_string(),
            },
            "",
            3100,
        )
        .expect("exec");
        assert!(bun.contains("/custom/bun"));
        assert!(bun.contains("--port 3100"));
    }

    #[test]
    fn resolve_exec_start_uses_env_bun_for_explicit_bun_command() {
        let _guard = env_lock().lock().expect("env lock poisoned");
        std::env::set_var("NANOSCALE_BUN_BIN", "  /env/bun  ");

        let exec = SystemdGenerator::resolve_exec_start(
            "/src",
            &AppRuntime::StandaloneNode,
            "bun run start",
            9999,
        )
        .expect("exec");
        assert!(exec.starts_with("/env/bun"));
        assert!(exec.contains("run start"));

        std::env::remove_var("NANOSCALE_BUN_BIN");
    }

    #[test]
    fn socket_template_contains_listen_port() {
        let template = SystemdGenerator::socket_template("nanoscale-p1", 3100);
        assert!(template.contains("ListenStream=127.0.0.1:3100"));
    }

    #[test]
    fn backend_port_offsets_by_10k() {
        assert_eq!(backend_port(3100).expect("backend_port"), 13_100);
    }
}
