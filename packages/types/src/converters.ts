import type {
  ApiAuthResponse,
  ApiBroadcastRequest,
  ApiCreateChannelRequest,
  ApiListenRequest,
  ApiListenResponse,
  ApiMineResponse,
  ApiNearbyChannel,
  ApiNearbyResponse,
  ApiScheduleEntry,
  ApiScheduleResponse,
  ApiUserPublic,
} from "./wire.js";
import type {
  AuthSession,
  BroadcastSession,
  Channel,
  CreateChannelInput,
  ListenSession,
  OwnedChannel,
  ScheduleEntry,
  User,
} from "./domain.js";

export function toUser(user: ApiUserPublic): User {
  return {
    id: user.id,
    username: user.username,
    isPro: user.is_pro,
  };
}

export function toAuthSession(response: ApiAuthResponse): AuthSession {
  return {
    token: response.token,
    user: toUser(response.user),
  };
}

export function toChannel(channel: ApiNearbyChannel): Channel {
  return {
    id: channel.id,
    frequency: channel.frequency,
    name: channel.name,
    status: channel.status,
    distanceM: channel.distance_m,
    listenerCount: channel.listener_count,
    owner: {
      username: channel.owner.username,
    },
  };
}

export function toNearbyChannels(response: ApiNearbyResponse): Channel[] {
  return response.channels.map(toChannel);
}

export function toOwnedChannels(response: ApiMineResponse): OwnedChannel[] {
  return response.channels.map((channel: ApiMineResponse["channels"][number]) => ({
    id: channel.id,
    frequency: channel.frequency,
    name: channel.name,
    status: channel.status,
    radiusM: channel.radius_m,
  }));
}

export function toCreateChannelRequest(
  channel: CreateChannelInput,
): ApiCreateChannelRequest {
  return {
    frequency: channel.frequency,
    name: channel.name,
    lat: channel.lat,
    lng: channel.lng,
    radius_m: channel.radiusM,
  };
}

export function toBroadcastRequest(channelId: string): ApiBroadcastRequest {
  return {
    channel_id: channelId,
  };
}

export function toListenRequest(channelId: string): ApiListenRequest {
  return {
    channel_id: channelId,
  };
}

export function toListenSession(response: ApiListenResponse): ListenSession {
  return {
    livekitToken: response.livekit_token,
    roomName: response.room_name,
    channel: {
      id: response.channel.id,
      frequency: response.channel.frequency,
      name: response.channel.name,
      listenerCount: response.channel.listener_count,
    },
  };
}

export function toBroadcastSession(response: {
  livekit_token: string;
  room_name: string;
}): BroadcastSession {
  return {
    livekitToken: response.livekit_token,
    roomName: response.room_name,
  };
}

export function toScheduleEntry(entry: ApiScheduleEntry): ScheduleEntry {
  return {
    showName: entry.show_name,
    hostName: entry.host_name,
    startTime: entry.start_time,
    dayOfWeek: entry.day_of_week,
    durationMinutes: entry.duration_minutes,
  };
}

export function toSchedule(response: ApiScheduleResponse): ScheduleEntry[] {
  return response.schedule.map(toScheduleEntry);
}
