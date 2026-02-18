import { clientApiBaseUrl } from "@/lib/api-base-url";

import type { ServerStatsResponse } from "@/lib/server-stats-api";

export async function fetchServerStatsClient(serverId: string): Promise<ServerStatsResponse> {
  const response = await fetch(`${clientApiBaseUrl()}/api/servers/${serverId}/stats`, {
    method: "GET",
    credentials: "include",
  });

  if (!response.ok) {
    throw new Error(`Unable to load server stats (HTTP ${response.status})`);
  }

  return (await response.json()) as ServerStatsResponse;
}
