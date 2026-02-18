use std::path::Path;
use std::process::Command;

use anyhow::Result;

use crate::system::PrivilegeWrapper;

const SYSTEMD_PATH: &str = "/etc/systemd/system";
const NGINX_ENABLED_PATH: &str = "/etc/nginx/sites-enabled";
const PROJECT_SITES_PATH: &str = "/opt/nanoscale/sites";
const PROJECT_TMP_PATH: &str = "/opt/nanoscale/tmp";

#[derive(Debug)]
pub struct Teardown;

impl Teardown {
    /// Deletes systemd units, nginx config, site directories, and the project user.
    ///
    /// # Errors
    /// Returns an error if a required privileged deletion or reload command fails.
    pub fn delete_project(project_id: &str, privilege_wrapper: &PrivilegeWrapper) -> Result<()> {
        let service_name = format!("nanoscale-{project_id}.service");
        let socket_name = format!("nanoscale-{project_id}.socket");

        let service_unit_path = format!("{SYSTEMD_PATH}/{service_name}");
        let socket_unit_path = format!("{SYSTEMD_PATH}/{socket_name}");
        let service_wants_path = format!("{SYSTEMD_PATH}/multi-user.target.wants/{service_name}");
        let socket_wants_path = format!("{SYSTEMD_PATH}/sockets.target.wants/{socket_name}");
        let nginx_conf_path = format!("{NGINX_ENABLED_PATH}/nanoscale-{project_id}.conf");
        let project_sites_path = format!("{PROJECT_SITES_PATH}/{project_id}");
        let project_tmp_path = format!("{PROJECT_TMP_PATH}/{project_id}");

        let _ = privilege_wrapper.run("/usr/bin/systemctl", &["stop", &service_name]);
        let _ = privilege_wrapper.run("/usr/bin/systemctl", &["disable", "--now", &service_name]);

        Self::remove_file_if_exists(privilege_wrapper, &service_unit_path)?;
        Self::remove_file_if_exists(privilege_wrapper, &socket_unit_path)?;
        Self::remove_file_if_exists(privilege_wrapper, &service_wants_path)?;
        Self::remove_file_if_exists(privilege_wrapper, &socket_wants_path)?;

        privilege_wrapper.run("/usr/bin/systemctl", &["daemon-reload"])?;

        let nginx_removed = Self::remove_file_if_exists(privilege_wrapper, &nginx_conf_path)?;
        if nginx_removed {
            privilege_wrapper.run("/usr/sbin/service", &["nginx", "reload"])?;
        }

        Self::remove_directory_if_exists(privilege_wrapper, &project_sites_path)?;
        Self::remove_directory_if_exists(privilege_wrapper, &project_tmp_path)?;

        Self::remove_project_user(project_id, privilege_wrapper)?;

        Ok(())
    }

    fn remove_file_if_exists(privilege_wrapper: &PrivilegeWrapper, path: &str) -> Result<bool> {
        if !Path::new(path).exists() {
            return Ok(false);
        }

        privilege_wrapper.run("/usr/bin/rm", &["-f", path])?;
        Ok(true)
    }

    fn remove_directory_if_exists(privilege_wrapper: &PrivilegeWrapper, path: &str) -> Result<()> {
        if !Path::new(path).exists() {
            return Ok(());
        }

        privilege_wrapper.run("/usr/bin/rm", &["-rf", path])?;
        Ok(())
    }

    fn remove_project_user(project_id: &str, privilege_wrapper: &PrivilegeWrapper) -> Result<()> {
        let username = format!("nanoscale-{project_id}");
        let user_exists = Command::new("/usr/bin/id")
            .arg("-u")
            .arg(&username)
            .status()
            .map(|status| status.success())
            .unwrap_or(false);

        if user_exists {
            privilege_wrapper.run("/usr/sbin/userdel", &[&username])?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remove_file_if_exists_returns_false_when_missing_without_sudo() {
        let wrapper = PrivilegeWrapper::new();
        let tempdir = tempfile::tempdir().expect("tempdir");
        let missing_path = tempdir.path().join("missing.service");
        let result = Teardown::remove_file_if_exists(&wrapper, &missing_path.to_string_lossy())
            .expect("remove_file_if_exists");
        assert!(!result);
    }

    #[test]
    fn remove_directory_if_exists_is_noop_when_missing_without_sudo() {
        let wrapper = PrivilegeWrapper::new();
        let tempdir = tempfile::tempdir().expect("tempdir");
        let missing_path = tempdir.path().join("missing-dir");
        Teardown::remove_directory_if_exists(&wrapper, &missing_path.to_string_lossy())
            .expect("remove_directory_if_exists");
    }
}
