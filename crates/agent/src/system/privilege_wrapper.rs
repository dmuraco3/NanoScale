use std::collections::HashSet;
use std::path::Path;
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

        Self::validate_command_args(binary_path, args)?;

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

    fn validate_command_args(binary_path: &str, args: &[&str]) -> Result<()> {
        match binary_path {
            SYSTEMCTL_BIN => Self::validate_systemctl_args(args),
            SERVICE_BIN => Self::validate_service_args(args),
            USERADD_BIN => Self::validate_useradd_args(args),
            USERDEL_BIN => Self::validate_userdel_args(args),
            CERTBOT_BIN => Self::validate_certbot_args(args),
            MV_BIN => Self::validate_mv_args(args),
            RM_BIN => Self::validate_rm_args(args),
            CHOWN_BIN => Self::validate_chown_args(args),
            FALLOCATE_BIN => Self::validate_fallocate_args(args),
            _ => Err(anyhow!("unsupported binary path: {binary_path}")),
        }
    }

    fn validate_systemctl_args(args: &[&str]) -> Result<()> {
        if args == ["daemon-reload"] {
            return Ok(());
        }

        if args.len() == 3
            && matches!(args[0], "enable" | "disable")
            && args[1] == "--now"
            && args[2].starts_with("nanoscale-")
            && args[2].ends_with(".service")
        {
            return Ok(());
        }

        if args == ["status", "nanoscale-agent"] {
            return Ok(());
        }

        if args.len() == 4
            && args[0] == "show"
            && args[1].starts_with("--property=")
            && args[2] == "--value"
            && args[3].starts_with("nanoscale-")
            && args[3].ends_with(".service")
        {
            return Ok(());
        }

        if args.len() == 3
            && args[0] == "show"
            && args[1].starts_with("--property=")
            && args[2].starts_with("nanoscale-")
            && args[2].ends_with(".service")
        {
            return Ok(());
        }

        if args.len() == 2
            && matches!(args[0], "start" | "stop" | "restart")
            && args[1].starts_with("nanoscale-")
        {
            return Ok(());
        }

        Err(anyhow!("systemctl arguments are not allowed: {args:?}"))
    }

    fn validate_service_args(args: &[&str]) -> Result<()> {
        if args == ["nginx", "reload"] {
            return Ok(());
        }

        Err(anyhow!("service arguments are not allowed: {args:?}"))
    }

    fn validate_useradd_args(args: &[&str]) -> Result<()> {
        if args.len() == 4
            && args[0] == "-r"
            && args[1] == "-s"
            && args[2] == "/bin/false"
            && args[3].starts_with("nanoscale-")
        {
            return Ok(());
        }

        Err(anyhow!("useradd arguments are not allowed: {args:?}"))
    }

    fn validate_userdel_args(args: &[&str]) -> Result<()> {
        if args.len() == 1 && args[0].starts_with("nanoscale-") {
            return Ok(());
        }

        Err(anyhow!("userdel arguments are not allowed: {args:?}"))
    }

    fn validate_certbot_args(args: &[&str]) -> Result<()> {
        if args.len() >= 2 && args[0] == "--nginx" {
            return Ok(());
        }

        if args.first().is_some_and(|value| *value == "certonly") {
            return Self::validate_certbot_certonly_webroot_args(args);
        }

        Err(anyhow!("certbot arguments are not allowed: {args:?}"))
    }

    fn validate_certbot_certonly_webroot_args(args: &[&str]) -> Result<()> {
        let mut has_webroot = false;
        let mut has_non_interactive = false;
        let mut has_agree_tos = false;
        let mut has_keep_until_expiring = false;
        let mut webroot_path: Option<&str> = None;
        let mut domain: Option<&str> = None;
        let mut email: Option<&str> = None;

        let mut i = 0_usize;
        while i < args.len() {
            match args[i] {
                "certonly" => {
                    i += 1;
                }
                "--webroot" => {
                    has_webroot = true;
                    i += 1;
                }
                "-w" => {
                    let value = args
                        .get(i + 1)
                        .ok_or_else(|| anyhow!("certbot -w requires a value"))?;
                    webroot_path = Some(value);
                    i += 2;
                }
                "-d" => {
                    let value = args
                        .get(i + 1)
                        .ok_or_else(|| anyhow!("certbot -d requires a value"))?;
                    domain = Some(value);
                    i += 2;
                }
                "--email" => {
                    let value = args
                        .get(i + 1)
                        .ok_or_else(|| anyhow!("certbot --email requires a value"))?;
                    email = Some(value);
                    i += 2;
                }
                "--non-interactive" => {
                    has_non_interactive = true;
                    i += 1;
                }
                "--agree-tos" => {
                    has_agree_tos = true;
                    i += 1;
                }
                "--keep-until-expiring" => {
                    has_keep_until_expiring = true;
                    i += 1;
                }
                other => {
                    return Err(anyhow!("certbot argument is not allowed: {other}"));
                }
            }
        }

        if !has_webroot || !has_non_interactive || !has_agree_tos || !has_keep_until_expiring {
            return Err(anyhow!(
                "certbot certonly args missing required flags: {args:?}"
            ));
        }

        let Some(webroot_path) = webroot_path else {
            return Err(anyhow!("certbot certonly must include -w"));
        };

        if webroot_path != "/opt/nanoscale/acme" {
            return Err(anyhow!(
                "certbot webroot path is not allowed: {webroot_path}"
            ));
        }

        let Some(domain) = domain else {
            return Err(anyhow!("certbot certonly must include -d"));
        };

        if domain.trim().is_empty()
            || !domain.contains('.')
            || !domain
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '-')
        {
            return Err(anyhow!("certbot domain is not allowed: {domain}"));
        }

        let Some(email) = email else {
            return Err(anyhow!("certbot certonly must include --email"));
        };

        if email.trim().is_empty() || email.contains(' ') || !email.contains('@') {
            return Err(anyhow!("certbot email is not allowed"));
        }

        Ok(())
    }

    fn validate_mv_args(args: &[&str]) -> Result<()> {
        if args.len() != 2 {
            return Err(anyhow!("mv requires source and destination paths"));
        }

        let source = args[0];
        let destination = args[1];

        let source_allowed = source.starts_with("/opt/nanoscale/tmp/nanoscale-")
            && (source.ends_with(".service")
                || source.ends_with(".socket")
                || Self::has_conf_extension(source));

        let destination_allowed = (destination.starts_with("/etc/systemd/system/nanoscale-")
            && (destination.ends_with(".service") || destination.ends_with(".socket")))
            || (destination.starts_with("/etc/nginx/sites-available/nanoscale-")
                && Self::has_conf_extension(destination))
            || (destination.starts_with("/etc/nginx/sites-enabled/nanoscale-")
                && Self::has_conf_extension(destination));

        if source_allowed && destination_allowed {
            return Ok(());
        }

        Err(anyhow!("mv arguments are not allowed: {args:?}"))
    }

    fn validate_chown_args(args: &[&str]) -> Result<()> {
        if args.len() != 3 || args[0] != "-R" {
            return Err(anyhow!("chown arguments are not allowed: {args:?}"));
        }

        let owner = args[1];
        let destination = args[2];

        let owner_allowed = if let Some((user, group)) = owner.split_once(':') {
            user.starts_with("nanoscale-") && group.starts_with("nanoscale-")
        } else {
            false
        };

        let destination_allowed = destination.starts_with("/opt/nanoscale/sites/nanoscale-")
            || destination.starts_with("/opt/nanoscale/sites/");

        if owner_allowed && destination_allowed {
            return Ok(());
        }

        Err(anyhow!("chown arguments are not allowed: {args:?}"))
    }

    fn validate_rm_args(args: &[&str]) -> Result<()> {
        if args.len() != 2 {
            return Err(anyhow!("rm requires exactly two arguments"));
        }

        let flag = args[0];
        let target = args[1];

        if flag == "-f" && Self::rm_file_target_allowed(target) {
            return Ok(());
        }

        if flag == "-rf" && Self::rm_directory_target_allowed(target) {
            return Ok(());
        }

        Err(anyhow!("rm arguments are not allowed: {args:?}"))
    }

    fn rm_file_target_allowed(target: &str) -> bool {
        (target.starts_with("/etc/systemd/system/nanoscale-")
            && (target.ends_with(".service") || target.ends_with(".socket")))
            || (target.starts_with("/etc/systemd/system/multi-user.target.wants/nanoscale-")
                && target.ends_with(".service"))
            || (target.starts_with("/etc/systemd/system/sockets.target.wants/nanoscale-")
                && target.ends_with(".socket"))
            || (target.starts_with("/etc/nginx/sites-enabled/nanoscale-")
                && Self::has_conf_extension(target))
    }

    fn rm_directory_target_allowed(target: &str) -> bool {
        (target.starts_with("/opt/nanoscale/sites/") || target.starts_with("/opt/nanoscale/tmp/"))
            && !target.contains("..")
    }

    fn validate_fallocate_args(args: &[&str]) -> Result<()> {
        if args == ["-l", "2G", "/opt/nanoscale/tmp/nanoscale.swap"] {
            return Ok(());
        }

        Err(anyhow!("fallocate arguments are not allowed: {args:?}"))
    }

    fn has_conf_extension(path: &str) -> bool {
        Path::new(path)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("conf"))
    }
}
