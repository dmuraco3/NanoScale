'use server';

import { headers } from "next/headers";

interface SetupAdminResult {
  ok: boolean;
  status: number;
}

export async function setupAdminAction(
  username: string,
  password: string,
): Promise<SetupAdminResult> {
  const requestHeaders = await headers();
  const cookie = requestHeaders.get("cookie") ?? "";
  const internalApiUrl = process.env.NANOSCALE_INTERNAL_API_URL ?? "http://127.0.0.1:4000";

  try {
    const response = await fetch(`${internalApiUrl}/api/auth/setup`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
        cookie,
      },
      body: JSON.stringify({ username, password }),
      cache: "no-store",
    });

    return {
      ok: response.ok,
      status: response.status,
    };
  } catch {
    return {
      ok: false,
      status: 0,
    };
  }
}
