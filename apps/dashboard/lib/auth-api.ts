import { headers } from "next/headers";

export interface AuthStatus {
  users_count: number;
  authenticated: boolean;
}

function apiBaseUrl(): string {
  return process.env.NEXT_PUBLIC_NANOSCALE_API_URL ?? "";
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
  return process.env.NEXT_PUBLIC_NANOSCALE_API_URL ?? "";
}
