use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Result};

use crate::system::PrivilegeWrapper;

use crate::deployment::tls::ACME_WEBROOT_PATH;

const TMP_BASE_PATH: &str = "/opt/nanoscale/tmp";
const NGINX_SITES_ENABLED: &str = "/etc/nginx/sites-enabled";

#[derive(Debug)]
pub struct NginxGenerator;

#[derive(Clone, Copy, Debug)]
pub enum NginxTlsMode<'a> {
    Disabled,
    Enabled { domain: &'a str },
}

impl NginxGenerator {
    pub fn generate_and_install(
        project_id: &str,
        port: u16,
        domain: Option<&str>,
        tls_mode: NginxTlsMode<'_>,
        privilege_wrapper: &PrivilegeWrapper,
    ) -> Result<()> {
        let site_name = format!("nanoscale-{project_id}");
        let server_name = Self::server_name(project_id, domain);
        let tmp_conf_enabled_path =
            PathBuf::from(format!("{TMP_BASE_PATH}/{site_name}.enabled.conf"));

        if let Some(parent_dir) = tmp_conf_enabled_path.parent() {
            fs::create_dir_all(parent_dir)?;
        }

        let conf_text = match tls_mode {
            NginxTlsMode::Disabled => Self::nginx_http_template(&server_name, port),
            NginxTlsMode::Enabled { domain } => {
                Self::nginx_https_template(&server_name, domain, port)
            }
        };
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

    fn server_name(project_id: &str, domain: Option<&str>) -> String {
        let compact_id = project_id.replace('-', "");
        let short_id = compact_id.chars().take(12).collect::<String>();
        let fallback = format!("ns-{short_id}.local");

        let domain = domain.map(str::trim).filter(|value| !value.is_empty());
        match domain {
            Some(domain) => format!("{domain} {fallback}"),
            None => fallback,
        }
    }

    fn nginx_http_template(server_name: &str, port: u16) -> String {
        format!(
            "server {{\n    listen 80;\n    server_name {server_name};\n\n    location ^~ /.well-known/acme-challenge/ {{\n        root {ACME_WEBROOT_PATH};\n    }}\n\n    location / {{\n        proxy_http_version 1.1;\n        proxy_set_header Host $host;\n        proxy_set_header X-Real-IP $remote_addr;\n        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;\n        proxy_set_header X-Forwarded-Proto $scheme;\n        proxy_pass http://127.0.0.1:{port};\n    }}\n}}\n"
        )
    }

    fn nginx_https_template(server_name: &str, domain: &str, port: u16) -> String {
        let cert_path = format!("/etc/letsencrypt/live/{domain}/fullchain.pem");
        let key_path = format!("/etc/letsencrypt/live/{domain}/privkey.pem");

        format!(
            "server {{\n    listen 80;\n    server_name {server_name};\n\n    location ^~ /.well-known/acme-challenge/ {{\n        root {ACME_WEBROOT_PATH};\n    }}\n\n    location / {{\n        return 301 https://$host$request_uri;\n    }}\n}}\n\nserver {{\n    listen 443 ssl;\n    server_name {server_name};\n\n    ssl_certificate {cert_path};\n    ssl_certificate_key {key_path};\n\n    location / {{\n        proxy_http_version 1.1;\n        proxy_set_header Host $host;\n        proxy_set_header X-Real-IP $remote_addr;\n        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;\n        proxy_set_header X-Forwarded-Proto $scheme;\n        proxy_pass http://127.0.0.1:{port};\n    }}\n}}\n"
        )
    }
}
