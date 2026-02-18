use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Default)]
pub struct StatsCache {
    by_server: HashMap<String, CachedServerSample>,
}

#[derive(Debug, Clone)]
pub struct ComputedRates {
    pub network_rx_bytes_per_sec: f64,
    pub network_tx_bytes_per_sec: f64,
    pub projects: HashMap<String, ComputedProjectRates>,
}

#[derive(Debug, Clone)]
pub struct ComputedProjectRates {
    pub cpu_usage_percent: f64,
    pub network_ingress_bytes_per_sec: f64,
    pub network_egress_bytes_per_sec: f64,
}

#[derive(Debug)]
struct CachedServerSample {
    at: Instant,
    network_rx_bytes_total: u64,
    network_tx_bytes_total: u64,
    projects: HashMap<String, CachedProjectSample>,
}

#[derive(Debug)]
struct CachedProjectSample {
    cpu_usage_nsec: u64,
    network_ingress_bytes: u64,
    network_egress_bytes: u64,
}

impl StatsCache {
    #[allow(clippy::cast_precision_loss, clippy::too_many_lines)]
    pub fn compute_and_update(
        &mut self,
        server_id: &str,
        now: Instant,
        cpu_cores: usize,
        network_received_bytes_total: u64,
        network_transmitted_bytes_total: u64,
        projects: &[(
            String,
            /* cpu_usage_nsec_total */ u64,
            /* net_ingress */ u64,
            /* net_egress */ u64,
        )],
    ) -> ComputedRates {
        let previous = self.by_server.get(server_id);
        let (received_bytes_per_sec, transmitted_bytes_per_sec, elapsed_secs) = match previous {
            Some(sample) => {
                let elapsed = now.saturating_duration_since(sample.at);
                let secs = elapsed.as_secs_f64();
                if secs <= 0.0 {
                    (0.0, 0.0, 0.0)
                } else {
                    (
                        delta_per_sec(
                            network_received_bytes_total,
                            sample.network_rx_bytes_total,
                            secs,
                        ),
                        delta_per_sec(
                            network_transmitted_bytes_total,
                            sample.network_tx_bytes_total,
                            secs,
                        ),
                        secs,
                    )
                }
            }
            None => (0.0, 0.0, 0.0),
        };

        let mut computed_projects = HashMap::new();
        if let Some(prev) = previous {
            if elapsed_secs > 0.0 {
                for (project_id, cpu_nsec, net_in, net_out) in projects {
                    if let Some(prev_project) = prev.projects.get(project_id) {
                        let cpu_delta = cpu_nsec.saturating_sub(prev_project.cpu_usage_nsec);

                        let cpu_usage_percent = if cpu_cores == 0 {
                            0.0
                        } else {
                            let denom = elapsed_secs * (cpu_cores as f64) * 1_000_000_000.0;
                            if denom <= 0.0 {
                                0.0
                            } else {
                                (cpu_delta as f64) / denom * 100.0
                            }
                        };

                        computed_projects.insert(
                            project_id.clone(),
                            ComputedProjectRates {
                                cpu_usage_percent,
                                network_ingress_bytes_per_sec: delta_per_sec(
                                    *net_in,
                                    prev_project.network_ingress_bytes,
                                    elapsed_secs,
                                ),
                                network_egress_bytes_per_sec: delta_per_sec(
                                    *net_out,
                                    prev_project.network_egress_bytes,
                                    elapsed_secs,
                                ),
                            },
                        );
                    } else {
                        computed_projects.insert(
                            project_id.clone(),
                            ComputedProjectRates {
                                cpu_usage_percent: 0.0,
                                network_ingress_bytes_per_sec: 0.0,
                                network_egress_bytes_per_sec: 0.0,
                            },
                        );
                    }
                }
            }
        } else {
            for (project_id, _cpu_nsec, _net_in, _net_out) in projects {
                computed_projects.insert(
                    project_id.clone(),
                    ComputedProjectRates {
                        cpu_usage_percent: 0.0,
                        network_ingress_bytes_per_sec: 0.0,
                        network_egress_bytes_per_sec: 0.0,
                    },
                );
            }
        }

        let mut next_projects = HashMap::new();
        for (project_id, cpu_nsec, net_in, net_out) in projects {
            next_projects.insert(
                project_id.clone(),
                CachedProjectSample {
                    cpu_usage_nsec: *cpu_nsec,
                    network_ingress_bytes: *net_in,
                    network_egress_bytes: *net_out,
                },
            );
        }

        self.by_server.insert(
            server_id.to_string(),
            CachedServerSample {
                at: now,
                network_rx_bytes_total: network_received_bytes_total,
                network_tx_bytes_total: network_transmitted_bytes_total,
                projects: next_projects,
            },
        );

        ComputedRates {
            network_rx_bytes_per_sec: received_bytes_per_sec,
            network_tx_bytes_per_sec: transmitted_bytes_per_sec,
            projects: computed_projects,
        }
    }
}

#[allow(clippy::cast_precision_loss)]
fn delta_per_sec(current: u64, previous: u64, elapsed_secs: f64) -> f64 {
    if elapsed_secs <= 0.0 {
        return 0.0;
    }
    let delta = current.saturating_sub(previous);
    (delta as f64) / elapsed_secs
}
