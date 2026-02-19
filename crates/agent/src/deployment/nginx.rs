use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, bail, Result};

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
    /// Generates an nginx site config and installs it into `sites-enabled`, then reloads nginx.
    ///
    /// # Errors
    /// Returns an error if the config cannot be written, the temp path is invalid, or privileged
    /// install/reload commands fail.
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
        let backend_port = backend_port(port).unwrap_or(port);
        let upstream_name = format!("nanoscale_upstream_{port}");
        format!(
            "upstream {upstream_name} {{\n    server 127.0.0.1:{backend_port};\n    server 127.0.0.1:{port} backup;\n}}\n\nserver {{\n    listen 80;\n    server_name {server_name};\n\n    location ^~ /.well-known/acme-challenge/ {{\n        root {ACME_WEBROOT_PATH};\n    }}\n\n    location / {{\n        proxy_http_version 1.1;\n        proxy_set_header Host $host;\n        proxy_set_header X-Real-IP $remote_addr;\n        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;\n        proxy_set_header X-Forwarded-Proto $scheme;\n        proxy_next_upstream error timeout http_502 http_503 http_504;\n        proxy_next_upstream_tries 10;\n        proxy_next_upstream_timeout 10s;\n        proxy_connect_timeout 1s;\n        proxy_pass http://{upstream_name};\n    }}\n}}\n"
        )
    }

    fn nginx_https_template(server_name: &str, domain: &str, port: u16) -> String {
        let cert_path = format!("/etc/letsencrypt/live/{domain}/fullchain.pem");
        let key_path = format!("/etc/letsencrypt/live/{domain}/privkey.pem");
        let backend_port = backend_port(port).unwrap_or(port);
        let upstream_name = format!("nanoscale_upstream_{port}");

        format!(
            "upstream {upstream_name} {{\n    server 127.0.0.1:{backend_port};\n    server 127.0.0.1:{port} backup;\n}}\n\nserver {{\n    listen 80;\n    server_name {server_name};\n\n    location ^~ /.well-known/acme-challenge/ {{\n        root {ACME_WEBROOT_PATH};\n    }}\n\n    location / {{\n        return 301 https://$host$request_uri;\n    }}\n}}\n\nserver {{\n    listen 443 ssl;\n    server_name {server_name};\n\n    ssl_certificate {cert_path};\n    ssl_certificate_key {key_path};\n\n    location / {{\n        proxy_http_version 1.1;\n        proxy_set_header Host $host;\n        proxy_set_header X-Real-IP $remote_addr;\n        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;\n        proxy_set_header X-Forwarded-Proto $scheme;\n        proxy_next_upstream error timeout http_502 http_503 http_504;\n        proxy_next_upstream_tries 10;\n        proxy_next_upstream_timeout 10s;\n        proxy_connect_timeout 1s;\n        proxy_pass http://{upstream_name};\n    }}\n}}\n"
        )
    }
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

    #[test]
    fn server_name_includes_domain_and_fallback() {
        let name = NginxGenerator::server_name(
            "123e4567-e89b-12d3-a456-426614174000",
            Some("app.example.com"),
        );
        assert!(name.contains("app.example.com"));
        assert!(name.contains("ns-"));
        assert!(name.contains(".local"));
    }

    #[test]
    fn server_name_falls_back_when_domain_missing_or_blank() {
        let missing = NginxGenerator::server_name("p1", None);
        let blank = NginxGenerator::server_name("p1", Some("   "));
        assert_eq!(missing, blank);
        assert!(std::path::Path::new(&missing)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("local")));
    }

    #[test]
    fn http_template_contains_acme_root_and_proxy_pass() {
        let template = NginxGenerator::nginx_http_template("example", 3100);
        assert!(template.contains(ACME_WEBROOT_PATH));
        assert!(template.contains("server 127.0.0.1:13100"));
        assert!(template.contains("server 127.0.0.1:3100 backup"));
    }

    #[test]
    fn https_template_contains_cert_paths_and_redirect() {
        let template = NginxGenerator::nginx_https_template("example", "app.example.com", 3100);
        assert!(template.contains("/etc/letsencrypt/live/app.example.com/fullchain.pem"));
        assert!(template.contains("return 301 https://$host$request_uri"));
        assert!(template.contains("server 127.0.0.1:13100"));
        assert!(template.contains("server 127.0.0.1:3100 backup"));
    }
}
