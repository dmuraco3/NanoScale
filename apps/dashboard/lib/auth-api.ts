'use server';

import { headers } from "next/headers";

export interface AuthStatus {
  users_count: number;
  authenticated: boolean;
}

export async function fetchAuthStatus(): Promise<AuthStatus> {
  const requestHeaders = await headers();
  const cookie = requestHeaders.get("cookie") ?? "";
  const internalApiUrl = process.env.NANOSCALE_INTERNAL_API_URL ?? "http://127.0.0.1:4000";

  const response = await fetch(`${internalApiUrl}/api/auth/status`, {
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
