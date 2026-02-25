'use server';

import { headers } from "next/headers";

export interface GitHubStatus {
  enabled: boolean;
  configured: boolean;
  connected: boolean;
  github_login: string | null;
  app_install_url: string | null;
}

export async function fetchGitHubStatusServer(): Promise<GitHubStatus | null> {
  const requestHeaders = await headers();
  const cookie = requestHeaders.get("cookie") ?? "";
  const internalApiUrl = process.env.NANOSCALE_INTERNAL_API_URL ?? "http://127.0.0.1:4000";

  const response = await fetch(`${internalApiUrl}/api/integrations/github/status`, {
    method: "GET",
    headers: {
      cookie,
    },
    cache: "no-store",
  });

  if (!response.ok) {
    return null;
  }

  return (await response.json()) as GitHubStatus;
}
