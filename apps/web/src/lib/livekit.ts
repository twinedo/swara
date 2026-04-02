import { browser } from "$app/environment";
import { LIVEKIT_URL } from "$lib/config";
import { createLocalScreenTracks, type LocalTrack, Room, RoomEvent, Track } from "livekit-client";

let listenerVolume = 0.72;
const broadcastCaptureTracks = new WeakMap<Room, LocalTrack[]>();

export type BroadcastInput = "microphone" | "tab-audio" | "desktop-audio";

export interface PreparedBroadcastInput {
  source: Exclude<BroadcastInput, "microphone">;
  tracks: LocalTrack[];
}

function ensureLiveKitUrl(): string {
  if (!LIVEKIT_URL) {
    throw new Error("Set PUBLIC_LIVEKIT_URL before connecting to LiveKit.");
  }

  return LIVEKIT_URL;
}

function assertMicrophoneSupport(): void {
  if (!browser) {
    throw new Error("Broadcasting is only available in the browser.");
  }

  if (!window.isSecureContext) {
    throw new Error(
      "Microphone access is blocked on this HTTP page. Use localhost or HTTPS to broadcast from this device.",
    );
  }

  if (!navigator.mediaDevices?.getUserMedia) {
    throw new Error("This browser does not expose microphone capture on this page.");
  }
}

function assertDisplayAudioSupport(source: Exclude<BroadcastInput, "microphone">): void {
  if (!browser) {
    throw new Error("Broadcasting is only available in the browser.");
  }

  if (!window.isSecureContext) {
    throw new Error(
      "Desktop audio capture is blocked on this HTTP page. Use localhost or HTTPS to share tab or desktop audio.",
    );
  }

  if (!navigator.mediaDevices?.getDisplayMedia) {
    throw new Error("This browser does not support sharing tab or desktop audio.");
  }

  if (source === "desktop-audio" && !navigator.mediaDevices.getDisplayMedia) {
    throw new Error("Desktop audio capture is unavailable in this browser.");
  }
}

function stopBroadcastTracks(tracks: LocalTrack[]): void {
  tracks.forEach((track) => {
    track.stop();
  });
}

function cleanupBroadcastCapture(room: Room): void {
  const tracks = broadcastCaptureTracks.get(room);

  if (!tracks) {
    return;
  }

  stopBroadcastTracks(tracks);
  broadcastCaptureTracks.delete(room);
}

function ensureDisplayAudioTrack(
  tracks: LocalTrack[],
  source: Exclude<BroadcastInput, "microphone">,
): LocalTrack {
  const audioTrack = tracks.find((track) => track.kind === Track.Kind.Audio);

  if (audioTrack) {
    return audioTrack;
  }

  stopBroadcastTracks(tracks);

  if (source === "tab-audio") {
    throw new Error("No tab audio was shared. Pick a browser tab and enable audio sharing.");
  }

  throw new Error("No desktop audio was shared. Pick a screen, window, or tab with audio enabled.");
}

export function assertBroadcastMediaSupport(source: BroadcastInput = "microphone"): void {
  if (source === "microphone") {
    assertMicrophoneSupport();
    return;
  }

  assertDisplayAudioSupport(source);
}

export async function prepareBroadcastInput(
  source: BroadcastInput,
): Promise<PreparedBroadcastInput | null> {
  assertBroadcastMediaSupport(source);

  if (source === "microphone") {
    return null;
  }

  const tracks = await createLocalScreenTracks({
    audio: true,
    video: {
      displaySurface: source === "tab-audio" ? "browser" : "monitor",
    },
    selfBrowserSurface: "include",
    surfaceSwitching: "include",
    systemAudio: "include",
  });

  ensureDisplayAudioTrack(tracks, source);
  return { source, tracks };
}

export function releasePreparedBroadcastInput(input: PreparedBroadcastInput | null): void {
  if (!input) {
    return;
  }

  stopBroadcastTracks(input.tracks);
}

function cleanupAudioNodes(roomName?: string, trackKey?: string): void {
  if (!browser) {
    return;
  }

  document
    .querySelectorAll<HTMLAudioElement>("[data-swara-stream]")
    .forEach((element) => {
      const matchesRoom = !roomName || element.dataset.swaraStream === roomName;
      const matchesTrack = !trackKey || element.dataset.swaraTrack === trackKey;

      if (matchesRoom && matchesTrack) {
        element.remove();
      }
    });
}

function attachAudioNode(
  track: { attach: () => HTMLMediaElement },
  roomName: string,
  trackKey: string,
): void {
  if (!browser) {
    return;
  }

  cleanupAudioNodes(roomName, trackKey);
  const element = track.attach();

  if (element instanceof HTMLAudioElement) {
    element.autoplay = true;
    element.volume = listenerVolume;
    element.dataset.swaraStream = roomName;
    element.dataset.swaraTrack = trackKey;
    element.style.display = "none";
    document.body.appendChild(element);
  }
}

function getAttachedAudioNodes(roomName?: string): HTMLAudioElement[] {
  if (!browser) {
    return [];
  }

  return Array.from(document.querySelectorAll<HTMLAudioElement>("[data-swara-stream]")).filter(
    (element) => !roomName || element.dataset.swaraStream === roomName,
  );
}

export function setListenerVolume(nextVolume: number): void {
  listenerVolume = Math.min(1, Math.max(0, nextVolume));

  getAttachedAudioNodes().forEach((element) => {
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
      attachAudioNode(track, roomName, track.sid ?? `${roomName}-${Date.now()}`);
    }
  });

  room.on(RoomEvent.TrackUnsubscribed, (track) => {
    if (track.kind === Track.Kind.Audio) {
      cleanupAudioNodes(roomName, track.sid ?? undefined);
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
        attachAudioNode(track, roomName, publication.trackSid);
      }
    });
  });

  return room;
}

export async function joinAsBroadcaster(
  token: string,
  input: PreparedBroadcastInput | null,
  micEnabled = true,
): Promise<Room> {
  const room = new Room();
  room.on(RoomEvent.Disconnected, () => {
    cleanupBroadcastCapture(room);
  });

  await room.connect(ensureLiveKitUrl(), token);

  try {
    if (micEnabled) {
      await room.localParticipant.setMicrophoneEnabled(true);
    }

    if (input) {
      const audioTrack = ensureDisplayAudioTrack(input.tracks, input.source);
      broadcastCaptureTracks.set(room, input.tracks);
      await room.localParticipant.publishTrack(audioTrack, {
        source: Track.Source.ScreenShareAudio,
        name: input.source,
      });
    }
  } catch (error) {
    cleanupBroadcastCapture(room);
    await room.disconnect().catch(() => undefined);
    throw error;
  }

  return room;
}

export async function leaveRoom(room: Room | null): Promise<void> {
  if (!room) {
    return;
  }

  const roomName = room.name;
  cleanupBroadcastCapture(room);
  await room.disconnect();
  cleanupAudioNodes(roomName);
}

export async function setMicEnabled(room: Room, enabled: boolean): Promise<void> {
  await room.localParticipant.setMicrophoneEnabled(enabled);
}

export async function setBroadcastInputEnabled(
  room: Room,
  source: BroadcastInput,
  enabled: boolean,
): Promise<void> {
  if (source === "microphone") {
    await room.localParticipant.setMicrophoneEnabled(enabled);
    return;
  }

  const publication = room.localParticipant.getTrackPublication(Track.Source.ScreenShareAudio);
  const track = publication?.track;

  if (!track) {
    return;
  }

  if (enabled) {
    await track.unmute();
  } else {
    await track.mute();
  }
}

export async function resumeListenerAudio(room: Room | null): Promise<void> {
  if (!room) {
    return;
  }

  const nodes = getAttachedAudioNodes(room.name);
  await Promise.all(
    nodes.map(async (element) => {
      element.muted = false;
      element.volume = listenerVolume;
      await element.play().catch(() => undefined);
    }),
  );
}

export function pauseListenerAudio(room: Room | null): void {
  if (!room) {
    return;
  }

  getAttachedAudioNodes(room.name).forEach((element) => {
    element.pause();
  });
}
