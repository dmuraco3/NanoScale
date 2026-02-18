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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn bun_binary_prefers_env_override_even_if_nonexistent() {
        let _guard = env_lock().lock().expect("env lock poisoned");
        std::env::set_var("NANOSCALE_BUN_BIN", "  /custom/bun  ");
        let resolved = bun_binary().expect("bun binary resolution");
        assert_eq!(resolved, "/custom/bun");
        std::env::remove_var("NANOSCALE_BUN_BIN");
    }
}
