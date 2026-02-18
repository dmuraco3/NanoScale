use std::fs;

use anyhow::{anyhow, Context, Result};

use crate::system::PrivilegeWrapper;

pub const ACME_WEBROOT_PATH: &str = "/opt/nanoscale/acme";

pub struct TlsProvisioner;

impl TlsProvisioner {
    pub fn ensure_certificate(
        domain: &str,
        email: &str,
        privilege_wrapper: &PrivilegeWrapper,
    ) -> Result<()> {
        let domain = domain.trim();
        if domain.is_empty() {
            return Err(anyhow!("domain cannot be empty"));
        }

        let email = email.trim();
        if email.is_empty() {
            return Err(anyhow!("tls email cannot be empty"));
        }

        Self::ensure_acme_webroot()?;

        let args = [
            "certonly",
            "--webroot",
            "-w",
            ACME_WEBROOT_PATH,
            "-d",
            domain,
            "--non-interactive",
            "--agree-tos",
            "--keep-until-expiring",
            "--email",
            email,
        ];

        let _ = privilege_wrapper
            .run("/usr/bin/certbot", &args)
            .with_context(|| format!("certbot failed for domain {domain}"))?;

        Ok(())
    }

    fn ensure_acme_webroot() -> Result<()> {
        fs::create_dir_all(ACME_WEBROOT_PATH)
            .with_context(|| format!("failed to create ACME webroot: {ACME_WEBROOT_PATH}"))?;
        Ok(())
    }
}
