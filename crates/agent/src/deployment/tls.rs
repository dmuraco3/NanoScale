use std::fs;

use anyhow::{anyhow, Context, Result};

use crate::system::PrivilegeWrapper;

pub const ACME_WEBROOT_PATH: &str = "/opt/nanoscale/acme";

pub struct TlsProvisioner;

impl TlsProvisioner {
    /// .
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// 1) domain is 0-length string
    /// 2) tls email is 0-length string
    /// 3) certbot fails to generated a certificate
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_certificate_rejects_empty_inputs_before_side_effects() {
        let wrapper = PrivilegeWrapper::new();
        assert!(TlsProvisioner::ensure_certificate("", "a@b.com", &wrapper).is_err());
        assert!(TlsProvisioner::ensure_certificate("example.com", "", &wrapper).is_err());
        assert!(TlsProvisioner::ensure_certificate("   ", "   ", &wrapper).is_err());
    }
}
