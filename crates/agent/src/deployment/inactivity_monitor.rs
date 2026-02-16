use std::fs;
use std::process::Command;
use std::sync::Arc;
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
}

impl InactivityMonitor {
    pub fn new(projects: Arc<RwLock<Vec<MonitoredProject>>>) -> Self {
        Self { projects }
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
                    let check_result = tokio::task::spawn_blocking(move || {
                        should_stop_service(&project_for_check)
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

fn should_stop_service(project: &MonitoredProject) -> anyhow::Result<bool> {
    let privilege_wrapper = PrivilegeWrapper::new();

    let _ = privilege_wrapper.run(
        "/usr/bin/systemctl",
        &[
            "show",
            "--property=ActiveEnterTimestamp",
            &project.service_name,
        ],
    )?;

    let active_since_mono_output = privilege_wrapper.run(
        "/usr/bin/systemctl",
        &[
            "show",
            "--property=ActiveEnterTimestampMonotonic",
            "--value",
            &project.service_name,
        ],
    )?;

    let active_since_micros = String::from_utf8_lossy(&active_since_mono_output.stdout)
        .trim()
        .parse::<u64>()
        .unwrap_or(0);

    if active_since_micros == 0 {
        return Ok(false);
    }

    let current_uptime_seconds = read_uptime_seconds().unwrap_or(0);
    let current_uptime_micros = current_uptime_seconds.saturating_mul(1_000_000);

    let uptime_micros = current_uptime_micros.saturating_sub(active_since_micros);
    let uptime_seconds = uptime_micros / 1_000_000;

    let ss_output = Command::new("ss")
        .arg("-tn")
        .arg("src")
        .arg(format!(":{}", project.port))
        .output()?;

    let connection_lines = String::from_utf8_lossy(&ss_output.stdout)
        .lines()
        .count()
        .saturating_sub(1);

    Ok(connection_lines == 0 && uptime_seconds > INACTIVITY_THRESHOLD_SECONDS)
}

fn read_uptime_seconds() -> Option<u64> {
    let uptime_file = fs::read_to_string("/proc/uptime").ok()?;
    let seconds_string = uptime_file.split_whitespace().next()?;
    let whole_seconds = seconds_string.split('.').next()?;
    whole_seconds.parse::<u64>().ok()
}
