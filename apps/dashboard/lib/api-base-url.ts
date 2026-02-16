export function clientApiBaseUrl(): string {
  const configuredUrl = process.env.NEXT_PUBLIC_NANOSCALE_API_URL;
  if (configuredUrl && configuredUrl.length > 0) {
    try {
      const parsedUrl = new URL(configuredUrl);

      if (parsedUrl.hostname === "0.0.0.0") {
        if (typeof window !== "undefined") {
          const portSegment = parsedUrl.port.length > 0 ? `:${parsedUrl.port}` : "";
          return `${window.location.protocol}//${window.location.hostname}${portSegment}`;
        }

        const portSegment = parsedUrl.port.length > 0 ? `:${parsedUrl.port}` : ":4000";
        return `http://127.0.0.1${portSegment}`;
      }

      return configuredUrl;
    } catch {
      return configuredUrl;
    }
  }

  if (typeof window !== "undefined") {
    return "";
  }

  return process.env.NANOSCALE_INTERNAL_API_URL ?? "http://127.0.0.1:4000";
}
