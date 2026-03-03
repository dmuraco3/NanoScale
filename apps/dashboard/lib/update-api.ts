import { clientApiBaseUrl } from "@/lib/api-base-url";

export async function triggerUpdate(): Promise<void> {
  const response = await fetch(`${clientApiBaseUrl()}/api/admin/update`, {
    method: "POST",
    credentials: "include",
  });

  if (response.status === 401) {
    throw new Error("Your session has expired. Please log in again.");
  }

  if (response.status !== 202) {
    throw new Error("Unable to start update");
  }
}

export function pollHealthUntilReady(intervalMs = 2000): Promise<void> {
  return new Promise((resolve) => {
    let isPolling = false;

    const timer = window.setInterval(async () => {
      if (isPolling) {
        return;
      }

      isPolling = true;
      try {
        const response = await fetch(`${clientApiBaseUrl()}/api/health`, {
          cache: "no-store",
          credentials: "include",
        });

        if (response.status === 200) {
          window.clearInterval(timer);
          resolve();
        }
      } catch {
        // Expected while service is restarting.
      } finally {
        isPolling = false;
      }
    }, intervalMs);
  });
}
