use std::collections::HashSet;
use std::process::{Command, Output};

use anyhow::{anyhow, Result};

const SUDO_BIN: &str = "/usr/bin/sudo";
const SYSTEMCTL_BIN: &str = "/usr/bin/systemctl";
const SERVICE_BIN: &str = "/usr/sbin/service";
const USERADD_BIN: &str = "/usr/sbin/useradd";
const USERDEL_BIN: &str = "/usr/sbin/userdel";
const CERTBOT_BIN: &str = "/usr/bin/certbot";
const MV_BIN: &str = "/usr/bin/mv";
const RM_BIN: &str = "/usr/bin/rm";
const CHOWN_BIN: &str = "/usr/bin/chown";
const FALLOCATE_BIN: &str = "/usr/bin/fallocate";

#[derive(Debug)]
pub struct PrivilegeWrapper {
    allowed_binaries: HashSet<&'static str>,
}

mod certbot;
mod validators;

impl PrivilegeWrapper {
    pub fn new() -> Self {
        let allowed_binaries = HashSet::from([
            SYSTEMCTL_BIN,
            SERVICE_BIN,
            USERADD_BIN,
            USERDEL_BIN,
            CERTBOT_BIN,
            MV_BIN,
            RM_BIN,
            CHOWN_BIN,
            FALLOCATE_BIN,
        ]);

        Self { allowed_binaries }
    }

    pub fn run(&self, binary_path: &str, args: &[&str]) -> Result<Output> {
        if !self.allowed_binaries.contains(binary_path) {
            return Err(anyhow!("binary path is not allowed: {binary_path}"));
        }

        validators::validate_command_args(binary_path, args)?;

        let output = Command::new(SUDO_BIN)
            .arg("-n")
            .arg(binary_path)
            .args(args)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(anyhow!(
                "privileged command failed: {binary_path} {args:?}; stdout: {stdout}; stderr: {stderr}"
            ));
        }

        Ok(output)
    }
}
