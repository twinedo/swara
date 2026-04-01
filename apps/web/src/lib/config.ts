function fallbackLiveKitUrl(): string {
  if (typeof window === "undefined") {
    return "ws://127.0.0.1:7880";
  }

  const protocol = window.location.protocol === "https:" ? "wss" : "ws";
  return `${protocol}://${window.location.hostname}:7880`;
}

export const API_BASE_URL =
  import.meta.env.PUBLIC_API_BASE_URL?.replace(/\/$/, "") ??
  "";

export const LIVEKIT_URL =
  import.meta.env.PUBLIC_LIVEKIT_URL ?? fallbackLiveKitUrl();

export const LOCATION_STORAGE_KEY = "swara.location.v1";
export const SESSION_STORAGE_KEY = "swara.session.v1";
