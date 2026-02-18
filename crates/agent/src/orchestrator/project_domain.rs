use axum::http::StatusCode;

use super::OrchestratorState;

pub(super) async fn assigned_project_domain(
    state: &OrchestratorState,
    project_id: &str,
    project_name: &str,
) -> Result<Option<String>, (StatusCode, String)> {
    let Some(base_domain) = state.base_domain.as_deref() else {
        return Ok(None);
    };

    let label =
        slugify_project_name(project_name).map_err(|message| (StatusCode::BAD_REQUEST, message))?;
    let label = truncate_dns_label(&label);
    let mut fqdn = format!("{label}.{base_domain}");

    let in_use = state
        .db
        .is_project_domain_in_use(&fqdn)
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Unable to validate project domain uniqueness: {error}"),
            )
        })?;

    if in_use {
        let compact_id = project_id.replace('-', "");
        let suffix = compact_id.chars().take(6).collect::<String>();
        let adjusted_label = trim_label_for_suffix(&label, suffix.len());
        fqdn = format!("{adjusted_label}-{suffix}.{base_domain}");

        let adjusted_in_use = state
            .db
            .is_project_domain_in_use(&fqdn)
            .await
            .map_err(|error| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Unable to validate project domain uniqueness: {error}"),
                )
            })?;

        if adjusted_in_use {
            return Err((
                StatusCode::CONFLICT,
                "Unable to allocate unique subdomain for this project".to_string(),
            ));
        }
    }

    Ok(Some(fqdn))
}

fn slugify_project_name(name: &str) -> Result<String, String> {
    let mut slug = String::new();
    let mut previous_was_separator = false;

    for character in name.chars() {
        let lowercase = character.to_ascii_lowercase();
        if lowercase.is_ascii_alphanumeric() {
            slug.push(lowercase);
            previous_was_separator = false;
            continue;
        }

        if !previous_was_separator {
            slug.push('-');
            previous_was_separator = true;
        }
    }

    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        return Err("Project name cannot be converted into a valid subdomain".to_string());
    }

    Ok(slug)
}

fn trim_label_for_suffix(label: &str, suffix_len: usize) -> String {
    let max_prefix_len = 63_usize.saturating_sub(suffix_len + 1);
    let mut trimmed = label
        .chars()
        .take(max_prefix_len.max(1))
        .collect::<String>();

    while trimmed.ends_with('-') {
        let _ = trimmed.pop();
    }

    if trimmed.is_empty() {
        "project".to_string()
    } else {
        trimmed
    }
}

fn truncate_dns_label(label: &str) -> String {
    let mut truncated = label.chars().take(63).collect::<String>();
    while truncated.ends_with('-') {
        let _ = truncated.pop();
    }

    if truncated.is_empty() {
        "project".to_string()
    } else {
        truncated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_project_name_produces_dnsish_label() {
        assert_eq!(slugify_project_name("My App").expect("slug"), "my-app");
        assert_eq!(
            slugify_project_name("Hello__World!!").expect("slug"),
            "hello-world"
        );
        assert!(slugify_project_name("----").is_err());
        assert!(slugify_project_name("   ").is_err());
    }

    #[test]
    fn truncate_dns_label_limits_length_and_avoids_trailing_dash() {
        let long = "a".repeat(80);
        let truncated = truncate_dns_label(&long);
        assert_eq!(truncated.len(), 63);

        let ends_with_dash = format!("{}-", "a".repeat(80));
        let truncated = truncate_dns_label(&ends_with_dash);
        assert!(truncated.len() <= 63);
        assert!(!truncated.ends_with('-'));
    }

    #[test]
    fn trim_label_for_suffix_keeps_space_for_suffix() {
        let label = "a".repeat(63);
        let trimmed = trim_label_for_suffix(&label, 6);
        assert!(trimmed.len() <= 63 - (6 + 1));

        let trimmed = trim_label_for_suffix("-", 10);
        assert_eq!(trimmed, "project");
    }
}
