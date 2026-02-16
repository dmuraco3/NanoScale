'use server'

import { headers } from "next/headers";

import { clientApiBaseUrl } from "@/lib/api-base-url";

export interface ServerListItem {
  id: string;
  name: string;
  ip_address: string;
  status: string;
  ram_usage_percent: number;
}

function apiBaseUrl(): string {
  return clientApiBaseUrl();
}

export async function fetchServers(): Promise<ServerListItem[]> {
  const requestHeaders = await headers();
  const cookie = requestHeaders.get("cookie") ?? "";

  const response = await fetch(`${apiBaseUrl()}/api/servers`, {
    method: "GET",
    headers: {
      cookie,
    },
    cache: "no-store",
  });

  if (!response.ok) {
    return [];
  }

  return (await response.json()) as ServerListItem[];
}

export async function fetchServersClient(): Promise<ServerListItem[]> {
  const response = await fetch(`${clientApiBaseUrl()}/api/servers`, {
    method: "GET",
    credentials: "include",
  });

  if (!response.ok) {
    return [];
  }

  return (await response.json()) as ServerListItem[];
}

export async function generateJoinToken(): Promise<string> {
  const response = await fetch(`${clientApiBaseUrl()}/api/cluster/generate-token`, {
    method: "POST",
    credentials: "include",
  });

  if (!response.ok) {
    throw new Error("Unable to generate join token");
  }

  const payload = (await response.json()) as { token: string };
  return payload.token;
}
