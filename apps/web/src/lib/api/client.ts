import type {
  ApiAuthRequest,
  ApiAuthResponse,
  ApiCreateChannelResponse,
  ApiErrorResponse,
  ApiHeartbeatResponse,
  ApiListenResponse,
  ApiMineResponse,
  ApiNearbyResponse,
  ApiScheduleResponse,
  ApiStartBroadcastResponse,
  ApiStopBroadcastResponse,
  AuthSession,
  BroadcastSession,
  Channel,
  CreateChannelInput,
  LatLng,
  ListenSession,
  OwnedChannel,
  ScheduleEntry,
} from "@swara/types";
import {
  toAuthSession,
  toBroadcastRequest,
  toBroadcastSession,
  toCreateChannelRequest,
  toListenRequest,
  toListenSession,
  toNearbyChannels,
  toOwnedChannels,
  toSchedule,
} from "@swara/types";
import { get } from "svelte/store";

import { API_BASE_URL } from "$lib/config";
import { authToken } from "$lib/stores/user";

async function request<T>(
  path: string,
  init: RequestInit = {},
  needsAuth = false,
): Promise<T> {
  const headers = new Headers(init.headers);
  headers.set("Content-Type", "application/json");

  const token = get(authToken);
  if (needsAuth && token) {
    headers.set("Authorization", `Bearer ${token}`);
  }

  const response = await fetch(`${API_BASE_URL}${path}`, {
    ...init,
    headers,
  });

  if (!response.ok) {
    const payload = (await response.json().catch(() => null)) as ApiErrorResponse | null;
    throw new Error(payload?.error ?? `Request failed with ${response.status}`);
  }

  return (await response.json()) as T;
}

export async function login(credentials: ApiAuthRequest): Promise<AuthSession> {
  const response = await request<ApiAuthResponse>("/api/auth/login", {
    method: "POST",
    body: JSON.stringify(credentials),
  });

  return toAuthSession(response);
}

export async function register(credentials: ApiAuthRequest): Promise<AuthSession> {
  const response = await request<ApiAuthResponse>("/api/auth/register", {
    method: "POST",
    body: JSON.stringify(credentials),
  });

  return toAuthSession(response);
}

export async function fetchNearbyChannels(
  location: LatLng,
  radiusM: number,
): Promise<Channel[]> {
  const search = new URLSearchParams({
    lat: String(location.lat),
    lng: String(location.lng),
    radius: String(radiusM),
  });
  const response = await request<ApiNearbyResponse>(
    `/api/channels/nearby?${search.toString()}`,
  );

  return toNearbyChannels(response);
}

export async function fetchOwnedChannels(): Promise<OwnedChannel[]> {
  const response = await request<ApiMineResponse>("/api/channels/mine", {}, true);
  return toOwnedChannels(response);
}

export async function createChannel(input: CreateChannelInput): Promise<string> {
  const response = await request<ApiCreateChannelResponse>(
    "/api/channels",
    {
      method: "POST",
      body: JSON.stringify(toCreateChannelRequest(input)),
    },
    true,
  );

  return response.id;
}

export async function fetchSchedule(channelId: string): Promise<ScheduleEntry[]> {
  const response = await request<ApiScheduleResponse>(`/api/channels/${channelId}/schedule`);
  return toSchedule(response);
}

export async function startListening(channelId: string): Promise<ListenSession> {
  const response = await request<ApiListenResponse>(
    "/api/listen",
    {
      method: "POST",
      body: JSON.stringify(toListenRequest(channelId)),
    },
  );

  return toListenSession(response);
}

export async function startBroadcast(channelId: string): Promise<BroadcastSession> {
  const response = await request<ApiStartBroadcastResponse>(
    "/api/broadcast/start",
    {
      method: "POST",
      body: JSON.stringify(toBroadcastRequest(channelId)),
    },
    true,
  );

  return toBroadcastSession(response);
}

export async function stopBroadcast(channelId: string): Promise<boolean> {
  const response = await request<ApiStopBroadcastResponse>(
    "/api/broadcast/stop",
    {
      method: "POST",
      body: JSON.stringify(toBroadcastRequest(channelId)),
    },
    true,
  );

  return response.success;
}

export async function sendBroadcastHeartbeat(channelId: string): Promise<boolean> {
  const response = await request<ApiHeartbeatResponse>(
    "/api/broadcast/heartbeat",
    {
      method: "POST",
      body: JSON.stringify(toBroadcastRequest(channelId)),
    },
    true,
  );

  return response.ok;
}
