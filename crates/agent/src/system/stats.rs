use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use anyhow::Result;
use sysinfo::{Disks, Networks, ProcessesToUpdate, System};

#[derive(Clone, Debug)]
pub struct SystemTotalsSnapshot {
    pub cpu_usage_percent: f32,
    pub cpu_cores: usize,
    pub used_memory_bytes: u64,
    pub total_memory_bytes: u64,
    pub used_disk_bytes: u64,
    pub total_disk_bytes: u64,
    pub network_rx_bytes_total: u64,
    pub network_tx_bytes_total: u64,
}

#[derive(Clone, Debug, Default)]
pub struct ProjectCountersSnapshot {
    pub cpu_usage_nsec_total: u64,
    pub memory_current_bytes: u64,
    pub network_ingress_bytes_total: u64,
    pub network_egress_bytes_total: u64,
    pub disk_usage_bytes: u64,
}

#[derive(Clone, Debug)]
pub struct HostStatsSnapshot {
    pub totals: SystemTotalsSnapshot,
    pub projects: HashMap<String, ProjectCountersSnapshot>,
}

#[must_use]
pub fn collect_host_stats(project_ids: &[String]) -> HostStatsSnapshot {
    let mut system = System::new_all();
    system.refresh_cpu_usage();
    system.refresh_memory();

    let mut networks = Networks::new_with_refreshed_list();
    networks.refresh(true);

    let mut disks = Disks::new_with_refreshed_list();
    disks.refresh(true);

    system.refresh_processes(ProcessesToUpdate::All, true);

    let totals = collect_totals_snapshot(&system, &disks, &networks);
    let mut projects = HashMap::new();

    for project_id in project_ids {
        let counters = collect_project_counters(project_id, &system).unwrap_or_default();
        projects.insert(project_id.clone(), counters);
    }

    HostStatsSnapshot { totals, projects }
}

fn collect_totals_snapshot(
    system: &System,
    disks: &Disks,
    networks: &Networks,
) -> SystemTotalsSnapshot {
    let cpu_cores = system.cpus().len();
    let cpu_usage_percent = system.global_cpu_usage();
    let used_memory_bytes = system.used_memory();
    let total_memory_bytes = system.total_memory();

    let mut total_disk_bytes: u64 = 0;
    let mut used_disk_bytes: u64 = 0;
    for disk in disks.list() {
        total_disk_bytes = total_disk_bytes.saturating_add(disk.total_space());
        used_disk_bytes = used_disk_bytes
            .saturating_add(disk.total_space().saturating_sub(disk.available_space()));
    }

    let mut network_received_bytes_total: u64 = 0;
    let mut network_transmitted_bytes_total: u64 = 0;
    for (_name, data) in networks {
        network_received_bytes_total =
            network_received_bytes_total.saturating_add(data.total_received());
        network_transmitted_bytes_total =
            network_transmitted_bytes_total.saturating_add(data.total_transmitted());
    }

    SystemTotalsSnapshot {
        cpu_usage_percent,
        cpu_cores,
        used_memory_bytes,
        total_memory_bytes,
        used_disk_bytes,
        total_disk_bytes,
        network_rx_bytes_total: network_received_bytes_total,
        network_tx_bytes_total: network_transmitted_bytes_total,
    }
}

fn collect_project_counters(project_id: &str, system: &System) -> Result<ProjectCountersSnapshot> {
    let service_name = format!("nanoscale-{project_id}.service");
    let systemd_props = systemctl_show(
        &service_name,
        &[
            "MainPID",
            "CPUUsageNSec",
            "MemoryCurrent",
            "IPIngressBytes",
            "IPEgressBytes",
        ],
    )?;

    let main_pid: i32 = systemd_props
        .get("MainPID")
        .and_then(|value| value.parse::<i32>().ok())
        .unwrap_or(0);

    let cpu_usage_nsec_total = systemd_props
        .get("CPUUsageNSec")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);

    let mut memory_current_bytes = systemd_props
        .get("MemoryCurrent")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);

    if memory_current_bytes == 0 && main_pid > 0 {
        if let Ok(pid_usize) = usize::try_from(main_pid) {
            if let Some(process) = system.process(sysinfo::Pid::from(pid_usize)) {
                memory_current_bytes = process.memory();
            }
        }
    }

    let network_ingress_bytes_total = systemd_props
        .get("IPIngressBytes")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);
    let network_egress_bytes_total = systemd_props
        .get("IPEgressBytes")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);

    let disk_usage_bytes =
        directory_size_bytes(Path::new(&format!("/opt/nanoscale/sites/{project_id}")));

    Ok(ProjectCountersSnapshot {
        cpu_usage_nsec_total,
        memory_current_bytes,
        network_ingress_bytes_total,
        network_egress_bytes_total,
        disk_usage_bytes,
    })
}

fn systemctl_show(unit: &str, properties: &[&str]) -> Result<HashMap<String, String>> {
    let mut args = vec!["show", unit];
    for property in properties {
        args.push("--property");
        args.push(property);
    }

    let output = Command::new("/usr/bin/systemctl").args(&args).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut map = HashMap::new();
    for line in stdout.lines() {
        if let Some((key, value)) = line.split_once('=') {
            map.insert(key.to_string(), value.to_string());
        }
    }

    Ok(map)
}

fn directory_size_bytes(root: &Path) -> u64 {
    if !root.exists() {
        return 0;
    }

    let mut total: u64 = 0;
    let mut stack = vec![root.to_path_buf()];
    while let Some(path) = stack.pop() {
        let Ok(metadata) = std::fs::symlink_metadata(&path) else {
            continue;
        };

        if metadata.is_file() {
            total = total.saturating_add(metadata.len());
            continue;
        }

        if metadata.is_dir() {
            let Ok(entries) = std::fs::read_dir(&path) else {
                continue;
            };

            for entry in entries {
                let Ok(entry) = entry else {
                    continue;
                };
                stack.push(entry.path());
            }
        }
    }

    total
}
