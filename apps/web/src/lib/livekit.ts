import { browser } from "$app/environment";
import { LIVEKIT_URL } from "$lib/config";
import { Room, RoomEvent, Track } from "livekit-client";

let listenerVolume = 0.72;

function ensureLiveKitUrl(): string {
  if (!LIVEKIT_URL) {
    throw new Error("Set PUBLIC_LIVEKIT_URL before connecting to LiveKit.");
  }

  return LIVEKIT_URL;
}

function cleanupAudioNodes(roomName?: string): void {
  if (!browser) {
    return;
  }

  document
    .querySelectorAll<HTMLAudioElement>("[data-swara-stream]")
    .forEach((element) => {
      if (!roomName || element.dataset.swaraStream === roomName) {
        element.remove();
      }
    });
}

function attachAudioNode(track: { attach: () => HTMLMediaElement }, roomName: string): void {
  if (!browser) {
    return;
  }

  cleanupAudioNodes(roomName);
  const element = track.attach();

  if (element instanceof HTMLAudioElement) {
    element.autoplay = true;
    element.volume = listenerVolume;
    element.dataset.swaraStream = roomName;
    element.style.display = "none";
    document.body.appendChild(element);
  }
}

export function setListenerVolume(nextVolume: number): void {
  listenerVolume = Math.min(1, Math.max(0, nextVolume));

  if (!browser) {
    return;
  }

  document
    .querySelectorAll<HTMLAudioElement>("[data-swara-stream]")
    .forEach((element) => {
      element.volume = listenerVolume;
    });
}

export async function joinAsListener(token: string, roomName: string): Promise<Room> {
  cleanupAudioNodes(roomName);

  const room = new Room({
    adaptiveStream: true,
    dynacast: true,
  });

  room.on(RoomEvent.TrackSubscribed, (track) => {
    if (track.kind === Track.Kind.Audio) {
      attachAudioNode(track, roomName);
    }
  });

  room.on(RoomEvent.Disconnected, () => {
    cleanupAudioNodes(roomName);
  });

  await room.connect(ensureLiveKitUrl(), token);

  room.remoteParticipants.forEach((participant) => {
    participant.trackPublications.forEach((publication) => {
      const track = publication.track;
      if (track && track.kind === Track.Kind.Audio) {
        attachAudioNode(track, roomName);
      }
    });
  });

  return room;
}

export async function joinAsBroadcaster(token: string): Promise<Room> {
  const room = new Room();
  await room.connect(ensureLiveKitUrl(), token);
  await room.localParticipant.setMicrophoneEnabled(true);
  return room;
}

export async function leaveRoom(room: Room | null): Promise<void> {
  if (!room) {
    return;
  }

  const roomName = room.name;
  await room.disconnect();
  cleanupAudioNodes(roomName);
}

export async function setMicEnabled(room: Room, enabled: boolean): Promise<void> {
  await room.localParticipant.setMicrophoneEnabled(enabled);
}
