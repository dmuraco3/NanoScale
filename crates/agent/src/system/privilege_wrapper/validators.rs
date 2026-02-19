use std::path::Path;

use anyhow::{anyhow, Result};

use super::{
    certbot, CHOWN_BIN, FALLOCATE_BIN, MV_BIN, RM_BIN, SERVICE_BIN, SYSTEMCTL_BIN, USERADD_BIN,
    USERDEL_BIN,
};

pub(super) fn validate_command_args(binary_path: &str, args: &[&str]) -> Result<()> {
    match binary_path {
        SYSTEMCTL_BIN => validate_systemctl_args(args),
        SERVICE_BIN => validate_service_args(args),
        USERADD_BIN => validate_useradd_args(args),
        USERDEL_BIN => validate_userdel_args(args),
        super::CERTBOT_BIN => certbot::validate_certbot_args(args),
        MV_BIN => validate_mv_args(args),
        RM_BIN => validate_rm_args(args),
        CHOWN_BIN => validate_chown_args(args),
        FALLOCATE_BIN => validate_fallocate_args(args),
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
        && (args[2].ends_with(".service") || args[2].ends_with(".socket"))
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
        && (args[3].ends_with(".service") || args[3].ends_with(".socket"))
    {
        return Ok(());
    }

    if args.len() == 3
        && args[0] == "show"
        && args[1].starts_with("--property=")
        && args[2].starts_with("nanoscale-")
        && (args[2].ends_with(".service") || args[2].ends_with(".socket"))
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

fn validate_mv_args(args: &[&str]) -> Result<()> {
    if args.len() != 2 {
        return Err(anyhow!("mv requires source and destination paths"));
    }

    let source = args[0];
    let destination = args[1];

    let source_allowed = source.starts_with("/opt/nanoscale/tmp/nanoscale-")
        && (source.ends_with(".service")
            || source.ends_with(".socket")
            || has_conf_extension(source));

    let destination_allowed = (destination.starts_with("/etc/systemd/system/nanoscale-")
        && (destination.ends_with(".service") || destination.ends_with(".socket")))
        || (destination.starts_with("/etc/nginx/sites-available/nanoscale-")
            && has_conf_extension(destination))
        || (destination.starts_with("/etc/nginx/sites-enabled/nanoscale-")
            && has_conf_extension(destination));

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

    if flag == "-f" && rm_file_target_allowed(target) {
        return Ok(());
    }

    if flag == "-rf" && rm_directory_target_allowed(target) {
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
        || (target.starts_with("/etc/nginx/sites-enabled/nanoscale-") && has_conf_extension(target))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_systemctl_allows_expected_patterns() {
        validate_command_args(SYSTEMCTL_BIN, &["daemon-reload"]).expect("daemon-reload");
        validate_command_args(SYSTEMCTL_BIN, &["enable", "--now", "nanoscale-p1.service"])
            .expect("enable service");
        validate_command_args(SYSTEMCTL_BIN, &["enable", "--now", "nanoscale-p1.socket"])
            .expect("enable socket");
        validate_command_args(SYSTEMCTL_BIN, &["status", "nanoscale-agent"]).expect("status");
        validate_command_args(
            SYSTEMCTL_BIN,
            &[
                "show",
                "--property=ActiveEnterTimestampMonotonic",
                "--value",
                "nanoscale-p1.service",
            ],
        )
        .expect("show --value");

        validate_command_args(
            SYSTEMCTL_BIN,
            &[
                "show",
                "--property=NConnections",
                "--value",
                "nanoscale-p1.socket",
            ],
        )
        .expect("show socket NConnections");
    }

    #[test]
    fn validate_systemctl_rejects_other_args() {
        assert!(validate_command_args(SYSTEMCTL_BIN, &["isolate", "rescue"]).is_err());
        assert!(validate_command_args(SYSTEMCTL_BIN, &["restart", "ssh"]).is_err());
    }

    #[test]
    fn validate_mv_and_rm_allow_only_expected_paths() {
        validate_command_args(
            MV_BIN,
            &[
                "/opt/nanoscale/tmp/nanoscale-p1.service",
                "/etc/systemd/system/nanoscale-p1.service",
            ],
        )
        .expect("mv service");

        validate_command_args(
            MV_BIN,
            &[
                "/opt/nanoscale/tmp/nanoscale-p1.enabled.conf",
                "/etc/nginx/sites-enabled/nanoscale-p1.conf",
            ],
        )
        .expect("mv nginx conf");

        validate_command_args(RM_BIN, &["-f", "/etc/systemd/system/nanoscale-p1.service"])
            .expect("rm service");

        assert!(validate_command_args(RM_BIN, &["-rf", "/etc/"]).is_err());
        assert!(validate_command_args(MV_BIN, &["/tmp/a", "/etc/passwd"]).is_err());
    }

    #[test]
    fn has_conf_extension_checks_case_insensitively() {
        assert!(has_conf_extension(
            "/etc/nginx/sites-enabled/nanoscale-x.conf"
        ));
        assert!(has_conf_extension(
            "/etc/nginx/sites-enabled/nanoscale-x.CONF"
        ));
        assert!(!has_conf_extension(
            "/etc/nginx/sites-enabled/nanoscale-x.txt"
        ));
    }
}
