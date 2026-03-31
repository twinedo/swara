export interface LatLng {
  lat: number;
  lng: number;
}

export type ChannelStatus = "live" | "offline" | (string & {});

export interface User {
  id: string;
  username: string;
  isPro: boolean;
}

export interface AuthSession {
  token: string;
  user: User;
}

export interface ChannelOwner {
  username: string;
}

export interface Channel {
  id: string;
  frequency: number;
  name: string;
  status: ChannelStatus;
  distanceM: number;
  listenerCount: number;
  owner: ChannelOwner;
}

export interface OwnedChannel {
  id: string;
  frequency: number;
  name: string;
  status: ChannelStatus;
  radiusM: number;
}

export interface CreateChannelInput {
  frequency: number;
  name: string;
  lat: number;
  lng: number;
  radiusM?: number;
}

export interface ListenChannelSummary {
  id: string;
  frequency: number;
  name: string;
  listenerCount: number;
}

export interface ListenSession {
  livekitToken: string;
  roomName: string;
  channel: ListenChannelSummary;
}

export interface BroadcastSession {
  livekitToken: string;
  roomName: string;
}

export interface ScheduleEntry {
  showName: string;
  hostName: string | null;
  startTime: string;
  dayOfWeek: number | null;
  durationMinutes: number | null;
}
