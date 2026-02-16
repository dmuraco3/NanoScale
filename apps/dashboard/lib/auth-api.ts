'use server';

import { headers } from "next/headers";

export interface AuthStatus {
  users_count: number;
  authenticated: boolean;
}

function apiBaseUrl(): string {
  const configuredUrl = process.env.NEXT_PUBLIC_NANOSCALE_API_URL;
  if (configuredUrl && configuredUrl.length > 0) {
    return configuredUrl;
  }

  if (typeof window !== "undefined") {
    return `${window.location.protocol}//${window.location.hostname}:4000`;
  }

  return "http://127.0.0.1:4000";
}

export async function fetchAuthStatus(): Promise<AuthStatus> {
  const requestHeaders = await headers();
  const cookie = requestHeaders.get("cookie") ?? "";

  const response = await fetch(`${apiBaseUrl()}/api/auth/status`, {
    method: "POST",
    headers: {
      "content-type": "application/json",
      cookie,
    },
    cache: "no-store",
  });

  if (!response.ok) {
    return { users_count: 1, authenticated: false };
  }

  const payload = (await response.json()) as AuthStatus;
  return payload;
}

export function clientApiBaseUrl(): string {
  return apiBaseUrl();
}
