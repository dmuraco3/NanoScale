use std::path::Path;

use anyhow::{bail, Result};

pub(super) fn bun_binary() -> Result<String> {
    if let Ok(configured_binary) = std::env::var("NANOSCALE_BUN_BIN") {
        let trimmed_binary = configured_binary.trim();
        if !trimmed_binary.is_empty() {
            return Ok(trimmed_binary.to_string());
        }
    }

    for candidate in ["/usr/bin/bun", "/bin/bun", "/usr/local/bin/bun"] {
        if Path::new(candidate).is_file() {
            return Ok(candidate.to_string());
        }
    }

    if let Ok(path_value) = std::env::var("PATH") {
        for path_entry in path_value.split(':') {
            if path_entry.is_empty() {
                continue;
            }

            let candidate_path = Path::new(path_entry).join("bun");
            if candidate_path.is_file() {
                return Ok(candidate_path.to_string_lossy().to_string());
            }
        }
    }

    let current_path = std::env::var("PATH").unwrap_or_default();
    bail!("bun binary not found; install bun or set NANOSCALE_BUN_BIN (PATH={current_path})")
}
