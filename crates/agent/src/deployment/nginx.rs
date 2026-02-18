use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Result};

use crate::system::PrivilegeWrapper;

const TMP_BASE_PATH: &str = "/opt/nanoscale/tmp";
const NGINX_SITES_ENABLED: &str = "/etc/nginx/sites-enabled";

#[derive(Debug)]
pub struct NginxGenerator;

impl NginxGenerator {
    pub fn generate_and_install(
        project_id: &str,
        port: u16,
        privilege_wrapper: &PrivilegeWrapper,
    ) -> Result<()> {
        let site_name = format!("nanoscale-{project_id}");
        let tmp_conf_enabled_path =
            PathBuf::from(format!("{TMP_BASE_PATH}/{site_name}.enabled.conf"));

        if let Some(parent_dir) = tmp_conf_enabled_path.parent() {
            fs::create_dir_all(parent_dir)?;
        }

        let conf_text = Self::nginx_template(&site_name, port);
        fs::write(&tmp_conf_enabled_path, conf_text)?;

        let target_enabled_conf_path = format!("{NGINX_SITES_ENABLED}/{site_name}.conf");

        let tmp_enabled_conf_string = tmp_conf_enabled_path
            .to_str()
            .ok_or_else(|| anyhow!("invalid nginx temp enabled path"))?;

        privilege_wrapper.run(
            "/usr/bin/mv",
            &[tmp_enabled_conf_string, &target_enabled_conf_path],
        )?;
        privilege_wrapper.run("/usr/sbin/service", &["nginx", "reload"])?;

        Ok(())
    }

    fn nginx_template(site_name: &str, port: u16) -> String {
        format!(
            "server {{\n    listen 80;\n    server_name {site_name}.local;\n\n    location / {{\n        proxy_http_version 1.1;\n        proxy_set_header Host $host;\n        proxy_set_header X-Real-IP $remote_addr;\n        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;\n        proxy_set_header X-Forwarded-Proto $scheme;\n        proxy_pass http://127.0.0.1:{port};\n    }}\n}}\n"
        )
    }
}
