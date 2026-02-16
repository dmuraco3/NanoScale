export function clientApiBaseUrl(): string {
  if (typeof window !== "undefined") {
    return "";
  }

  const configuredUrl = process.env.NEXT_PUBLIC_NANOSCALE_API_URL;
  if (configuredUrl && configuredUrl.length > 0) {
    try {
      const parsedUrl = new URL(configuredUrl);

      if (parsedUrl.hostname === "0.0.0.0") {
        const portSegment = parsedUrl.port.length > 0 ? `:${parsedUrl.port}` : ":4000";
        return `http://127.0.0.1${portSegment}`;
      }

      return configuredUrl;
    } catch {
      return configuredUrl;
    }
  }

  return process.env.NANOSCALE_INTERNAL_API_URL ?? "http://127.0.0.1:4000";
}
