export const API_BASE_URL =
  import.meta.env.PUBLIC_API_BASE_URL?.replace(/\/$/, "") ??
  "";

export const LIVEKIT_URL = import.meta.env.PUBLIC_LIVEKIT_URL ?? "ws://127.0.0.1:7880";

export const LOCATION_STORAGE_KEY = "swara.location.v1";
export const SESSION_STORAGE_KEY = "swara.session.v1";
