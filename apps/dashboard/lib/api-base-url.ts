export function clientApiBaseUrl(): string {
  const configuredUrl = process.env.NEXT_PUBLIC_NANOSCALE_API_URL;
  if (configuredUrl && configuredUrl.length > 0) {
    return configuredUrl;
  }

  if (typeof window !== "undefined") {
    return `${window.location.protocol}//${window.location.hostname}:4000`;
  }

  return "http://127.0.0.1:4000";
}
