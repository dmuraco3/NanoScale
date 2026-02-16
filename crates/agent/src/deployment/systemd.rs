use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};

use crate::system::PrivilegeWrapper;

const TMP_BASE_PATH: &str = "/opt/nanoscale/tmp";
const SYSTEMD_TARGET_PATH: &str = "/etc/systemd/system";

#[derive(Debug)]
pub struct SystemdGenerator;

impl SystemdGenerator {
    pub fn generate_and_install(
        project_id: &str,
        source_dir: &Path,
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

        let service_template = Self::service_template(&service_name, project_id, source_dir_string);
        let socket_template = Self::socket_template(&service_name);

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

    fn service_template(service_name: &str, project_id: &str, source_dir: &str) -> String {
        format!(
            "[Unit]\nDescription=NanoScale app service ({service_name})\nAfter=network.target\n\n[Service]\nType=simple\nUser=nanoscale-{project_id}\nGroup=nanoscale-{project_id}\nWorkingDirectory={source_dir}\nEnvironment=NODE_ENV=production\nExecStart=/usr/bin/node {source_dir}/server.js\nRestart=always\nRestartSec=2\n\n# SECURITY HARDENING\nProtectSystem=strict\nProtectHome=yes\nPrivateTmp=yes\nNoNewPrivileges=yes\nProtectProc=invisible\nReadWritePaths={source_dir}\n\n[Install]\nWantedBy=multi-user.target\n"
        )
    }

    fn socket_template(service_name: &str) -> String {
        format!(
            "[Unit]\nDescription=NanoScale app socket ({service_name})\nPartOf={service_name}.service\n\n[Socket]\nListenStream=127.0.0.1:3000\nNoDelay=true\n\n[Install]\nWantedBy=sockets.target\n"
        )
    }
}
