use anyhow::{anyhow, Result};

pub(super) fn validate_certbot_args(args: &[&str]) -> Result<()> {
    if args.len() >= 2 && args[0] == "--nginx" {
        return Ok(());
    }

    if args.first().is_some_and(|value| *value == "certonly") {
        return validate_certbot_certonly_webroot_args(args);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_certbot_args_allows_nginx_mode() {
        validate_certbot_args(&["--nginx", "-v"]).expect("--nginx with extra args allowed");
    }

    #[test]
    fn validate_certbot_args_allows_webroot_certonly_with_required_flags() {
        let args = [
            "certonly",
            "--webroot",
            "-w",
            "/opt/nanoscale/acme",
            "-d",
            "app.example.com",
            "--non-interactive",
            "--agree-tos",
            "--keep-until-expiring",
            "--email",
            "ops@example.com",
        ];
        validate_certbot_args(&args).expect("certonly webroot args allowed");
    }

    #[test]
    fn validate_certbot_args_rejects_missing_required_flags() {
        let args = ["certonly", "--webroot", "-w", "/opt/nanoscale/acme"];
        assert!(validate_certbot_args(&args).is_err());
    }

    #[test]
    fn validate_certbot_args_rejects_bad_domain_or_email() {
        let args = [
            "certonly",
            "--webroot",
            "-w",
            "/opt/nanoscale/acme",
            "-d",
            "not-a-domain",
            "--non-interactive",
            "--agree-tos",
            "--keep-until-expiring",
            "--email",
            "ops@example.com",
        ];
        assert!(validate_certbot_args(&args).is_err());

        let args = [
            "certonly",
            "--webroot",
            "-w",
            "/opt/nanoscale/acme",
            "-d",
            "app.example.com",
            "--non-interactive",
            "--agree-tos",
            "--keep-until-expiring",
            "--email",
            "not-an-email",
        ];
        assert!(validate_certbot_args(&args).is_err());
    }
}
