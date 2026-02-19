use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use tokio::sync::RwLock;

use crate::system::PrivilegeWrapper;

const INACTIVITY_INTERVAL_SECONDS: u64 = 60;
const INACTIVITY_THRESHOLD_SECONDS: u64 = 15 * 60;

#[derive(Clone, Debug)]
pub struct MonitoredProject {
    pub service_name: String,
    pub port: u16,
    pub scale_to_zero: bool,
}

#[derive(Clone, Debug)]
pub struct InactivityMonitor {
    projects: Arc<RwLock<Vec<MonitoredProject>>>,
    traffic_state: Arc<Mutex<HashMap<String, TrafficState>>>,
}

impl InactivityMonitor {
    pub fn new(projects: Arc<RwLock<Vec<MonitoredProject>>>) -> Self {
        Self {
            projects,
            traffic_state: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn spawn(self) {
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(Duration::from_secs(INACTIVITY_INTERVAL_SECONDS));

            loop {
                interval.tick().await;

                let projects = self.projects.read().await.clone();
                for project in projects {
                    if !project.scale_to_zero {
                        continue;
                    }

                    let project_for_check = project.clone();
                    let traffic_state = self.traffic_state.clone();
                    let check_result = tokio::task::spawn_blocking(move || {
                        should_stop_service(&project_for_check, &traffic_state)
                    })
                    .await;

                    match check_result {
                        Ok(Ok(true)) => {
                            let privilege_wrapper = PrivilegeWrapper::new();
                            if let Err(error) = privilege_wrapper
                                .run("/usr/bin/systemctl", &["stop", &project.service_name])
                            {
                                eprintln!(
                                    "scale-to-zero stop failed for {}: {error}",
                                    project.service_name
                                );
                            }
                        }
                        Ok(Ok(false)) => {}
                        Ok(Err(error)) => {
                            eprintln!(
                                "scale-to-zero check failed for {}: {error}",
                                project.service_name
                            );
                        }
                        Err(error) => {
                            eprintln!("scale-to-zero task join error: {error}");
                        }
                    }
                }
            }
        });
    }
}

fn should_stop_service(
    project: &MonitoredProject,
    traffic_state: &Arc<Mutex<HashMap<String, TrafficState>>>,
) -> anyhow::Result<bool> {
    let privilege_wrapper = PrivilegeWrapper::new();

    let active_state_output = privilege_wrapper.run(
        "/usr/bin/systemctl",
        &[
            "show",
            "--property=ActiveState",
            "--value",
            &project.service_name,
        ],
    )?;

    let active_state = String::from_utf8_lossy(&active_state_output.stdout)
        .trim()
        .to_string();

    if active_state != "active" {
        return Ok(false);
    }

    let now_uptime_seconds = read_uptime_seconds().unwrap_or(0);
    if now_uptime_seconds == 0 {
        return Ok(false);
    }

    let socket_unit_name = service_unit_to_socket_unit(&project.service_name);
    let nconnections = match read_socket_nconnections(&privilege_wrapper, &socket_unit_name) {
        Ok(value) => value,
        Err(error) => {
            eprintln!(
                "scale-to-zero read socket connections failed for {}: {error}",
                project.service_name
            );
            return Ok(false);
        }
    };

    let mut state_guard = traffic_state
        .lock()
        .map_err(|_| anyhow::anyhow!("traffic_state mutex poisoned"))?;

    let entry = state_guard
        .entry(project.service_name.clone())
        .or_insert_with(|| TrafficState {
            last_nconnections: nconnections,
            last_activity_uptime_seconds: now_uptime_seconds,
        });

    if nconnections != entry.last_nconnections {
        entry.last_nconnections = nconnections;
        entry.last_activity_uptime_seconds = now_uptime_seconds;
    }

    let inactive_for_seconds =
        now_uptime_seconds.saturating_sub(entry.last_activity_uptime_seconds);
    Ok(inactive_for_seconds > INACTIVITY_THRESHOLD_SECONDS)
}

#[derive(Clone, Copy, Debug)]
struct TrafficState {
    last_nconnections: u64,
    last_activity_uptime_seconds: u64,
}

fn service_unit_to_socket_unit(service_unit_name: &str) -> String {
    if let Some(stem) = service_unit_name.strip_suffix(".service") {
        format!("{stem}.socket")
    } else {
        format!("{service_unit_name}.socket")
    }
}

fn read_socket_nconnections(
    privilege_wrapper: &PrivilegeWrapper,
    socket_unit_name: &str,
) -> anyhow::Result<u64> {
    let output = privilege_wrapper.run(
        "/usr/bin/systemctl",
        &[
            "show",
            "--property=NConnections",
            "--value",
            socket_unit_name,
        ],
    )?;

    let raw = String::from_utf8_lossy(&output.stdout);
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        anyhow::bail!("empty NConnections output")
    }

    let parsed = trimmed
        .parse::<u64>()
        .map_err(|error| anyhow::anyhow!("invalid NConnections '{trimmed}': {error}"))?;
    Ok(parsed)
}

fn read_uptime_seconds() -> Option<u64> {
    let uptime_file = fs::read_to_string("/proc/uptime").ok()?;
    let seconds_string = uptime_file.split_whitespace().next()?;
    let whole_seconds = seconds_string.split('.').next()?;
    whole_seconds.parse::<u64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_uptime_seconds_is_non_panicking() {
        let _ = read_uptime_seconds();
    }
}
