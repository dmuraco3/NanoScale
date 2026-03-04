'use server';

import { headers } from "next/headers";

export interface UpdateStatus {
  current_version: string;
  latest_version: string;
  update_available: boolean;
}

export async function fetchUpdateStatusServer(): Promise<UpdateStatus | null> {
  const requestHeaders = await headers();
  const cookie = requestHeaders.get("cookie") ?? "";
  const internalApiUrl = process.env.NANOSCALE_INTERNAL_API_URL ?? "http://127.0.0.1:4000";

  const response = await fetch(`${internalApiUrl}/api/admin/update-status`, {
    method: "GET",
    headers: {
      cookie,
    },
    cache: "no-store",
  });

  if (!response.ok) {
    return null;
  }

  return (await response.json()) as UpdateStatus;
}
