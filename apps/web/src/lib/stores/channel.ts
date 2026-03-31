import type { Channel, OwnedChannel, ScheduleEntry } from "@swara/types";
import type { Room } from "livekit-client";
import { derived, writable } from "svelte/store";

export type PlaybackState = "idle" | "connecting" | "playing" | "paused" | "error";
export type BroadcastState =
  | "idle"
  | "connecting"
  | "live"
  | "stopping"
  | "error"
  | "interrupted";

export const nearbyChannels = writable<Channel[]>([]);
export const activeChannel = writable<Channel | null>(null);
export const activeSchedule = writable<ScheduleEntry[]>([]);
export const ownedChannel = writable<OwnedChannel | null>(null);
export const playbackState = writable<PlaybackState>("idle");
export const broadcastState = writable<BroadcastState>("idle");
export const listenerRoom = writable<Room | null>(null);
export const broadcasterRoom = writable<Room | null>(null);

export const liveChannels = derived(nearbyChannels, ($nearbyChannels) =>
  $nearbyChannels.filter((channel) => channel.status === "live"),
);
export const isOnAir = derived(broadcastState, ($broadcastState) => $broadcastState === "live");

export function setNearbyChannels(channels: Channel[]): void {
  nearbyChannels.set(channels);
}

export function setOwnedChannels(channels: OwnedChannel[]): void {
  ownedChannel.set(channels[0] ?? null);
}

export function selectChannel(channel: Channel | null): void {
  activeChannel.set(channel);
}

export function resetListeningState(): void {
  listenerRoom.set(null);
  playbackState.set("idle");
  activeSchedule.set([]);
}

export function resetBroadcastState(): void {
  broadcasterRoom.set(null);
  broadcastState.set("idle");
}
