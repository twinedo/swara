export interface ApiErrorResponse {
  error: string;
}

export interface ApiUserPublic {
  id: string;
  username: string;
  is_pro: boolean;
}

export interface ApiAuthRequest {
  username: string;
  password: string;
}

export interface ApiAuthResponse {
  token: string;
  user: ApiUserPublic;
}

export interface ApiOwnerSummary {
  username: string;
}

export interface ApiNearbyChannel {
  id: string;
  frequency: number;
  name: string;
  status: string;
  distance_m: number;
  listener_count: number;
  owner: ApiOwnerSummary;
}

export interface ApiNearbyResponse {
  channels: ApiNearbyChannel[];
}

export interface ApiCreateChannelRequest {
  frequency: number;
  name: string;
  lat: number;
  lng: number;
  radius_m?: number;
}

export interface ApiCreateChannelResponse {
  id: string;
}

export interface ApiMyChannel {
  id: string;
  frequency: number;
  name: string;
  status: string;
  radius_m: number;
}

export interface ApiMineResponse {
  channels: ApiMyChannel[];
}

export interface ApiBroadcastRequest {
  channel_id: string;
}

export interface ApiStartBroadcastResponse {
  livekit_token: string;
  room_name: string;
}

export interface ApiStopBroadcastResponse {
  success: boolean;
}

export interface ApiHeartbeatResponse {
  ok: boolean;
}

export interface ApiListenRequest {
  channel_id: string;
}

export interface ApiListenChannelSummary {
  id: string;
  frequency: number;
  name: string;
  listener_count: number;
}

export interface ApiListenResponse {
  livekit_token: string;
  room_name: string;
  channel: ApiListenChannelSummary;
}

export interface ApiScheduleEntry {
  show_name: string;
  host_name: string | null;
  start_time: string;
  day_of_week: number | null;
  duration_minutes: number | null;
}

export interface ApiScheduleResponse {
  schedule: ApiScheduleEntry[];
}
