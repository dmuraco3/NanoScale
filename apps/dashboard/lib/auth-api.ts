'use server';

import { headers } from "next/headers";

import { clientApiBaseUrl } from "@/lib/api-base-url";

export interface AuthStatus {
  users_count: number;
  authenticated: boolean;
}

export async function fetchAuthStatus(): Promise<AuthStatus> {
  const requestHeaders = await headers();
  const cookie = requestHeaders.get("cookie") ?? "";

  const response = await fetch(`${clientApiBaseUrl()}/api/auth/status`, {
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
