'use server';

import { headers } from "next/headers";

import { clientApiBaseUrl } from "@/lib/api-base-url";

export interface ServerTotalsStats {
  cpu_usage_percent: number;
  cpu_cores: number;
  used_memory_bytes: number;
  total_memory_bytes: number;
  used_disk_bytes: number;
  total_disk_bytes: number;
  network_rx_bytes_total: number;
  network_tx_bytes_total: number;
  network_rx_bytes_per_sec: number;
  network_tx_bytes_per_sec: number;
}

export interface ProjectStatsBreakdown {
  project_id: string;
  project_name: string;
  cpu_usage_percent: number;
  memory_current_bytes: number;
  disk_usage_bytes: number;
  network_ingress_bytes_total: number;
  network_egress_bytes_total: number;
  network_ingress_bytes_per_sec: number;
  network_egress_bytes_per_sec: number;
}

export interface ServerStatsResponse {
  server_id: string;
  sample_unix_ms: number;
  totals: ServerTotalsStats;
  projects: ProjectStatsBreakdown[];
}

export async function fetchServerStats(serverId: string): Promise<ServerStatsResponse | null> {
  const requestHeaders = await headers();
  const cookie = requestHeaders.get("cookie") ?? "";

  const response = await fetch(`${clientApiBaseUrl()}/api/servers/${serverId}/stats`, {
    method: "GET",
    headers: {
      cookie,
    },
    cache: "no-store",
  });

  if (!response.ok) {
    return null;
  }

  return (await response.json()) as ServerStatsResponse;
}
