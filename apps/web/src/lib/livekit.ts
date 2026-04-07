import { browser } from "$app/environment";
import { LIVEKIT_URL } from "$lib/config";
import { createLocalScreenTracks, type LocalTrack, Room, RoomEvent, Track } from "livekit-client";

let listenerVolume = 0.72;
const broadcastCaptureTracks = new WeakMap<Room, LocalTrack[]>();
const broadcastMixStates = new WeakMap<Room, BroadcastMixState>();

export type BroadcastInput = "microphone" | "tab-audio" | "desktop-audio";

export interface PreparedBroadcastInput {
  source: Exclude<BroadcastInput, "microphone">;
  tracks: LocalTrack[];
}

export interface BroadcasterJoinOptions {
  micEnabled?: boolean;
  micGain?: number;
  sourceGain?: number;
  deckGain?: number;
  deckElement?: HTMLMediaElement | null;
}

interface BroadcastAudioChannel {
  gainNode: GainNode;
  desiredGain: number;
  enabled: boolean;
  outputTrack: MediaStreamTrack;
  rawTrack: MediaStreamTrack;
  sourceNode: MediaStreamAudioSourceNode;
}

interface BroadcastProgramChannel {
  gainNode: GainNode;
  desiredGain: number;
  enabled: boolean;
  element?: HTMLMediaElement;
  disconnect: () => void;
}

interface BroadcastMixState {
  audioContext: AudioContext;
  programDestination: MediaStreamAudioDestinationNode;
  programTrack: MediaStreamTrack;
  channels: {
    microphone?: BroadcastAudioChannel;
    programSource?: BroadcastProgramChannel;
    musicDeck?: BroadcastProgramChannel;
  };
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
  const mixState = broadcastMixStates.get(room);

  if (mixState) {
    mixState.channels.microphone?.sourceNode.disconnect();
    mixState.channels.microphone?.gainNode.disconnect();
    mixState.channels.microphone?.outputTrack.stop();
    mixState.channels.microphone?.rawTrack.stop();
    mixState.channels.programSource?.disconnect();
    mixState.channels.musicDeck?.disconnect();
    mixState.programTrack.stop();
    void mixState.audioContext.close().catch(() => undefined);
    broadcastMixStates.delete(room);
  }

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

function clampBroadcastGain(nextGain: number): number {
  return Math.min(2, Math.max(0, nextGain));
}

function getAudioContextConstructor(): typeof AudioContext | undefined {
  if (!browser) {
    return undefined;
  }

  return (
    window.AudioContext ??
    (window as Window & typeof globalThis & { webkitAudioContext?: typeof AudioContext })
      .webkitAudioContext
  );
}

function createBroadcastMixState(): BroadcastMixState {
  const AudioContextConstructor = getAudioContextConstructor();

  if (!AudioContextConstructor) {
    throw new Error("This browser cannot build the broadcaster audio mixer.");
  }

  const audioContext = new AudioContextConstructor();
  const programDestination = audioContext.createMediaStreamDestination();
  const [programTrack] = programDestination.stream.getAudioTracks();

  if (!programTrack) {
    void audioContext.close().catch(() => undefined);
    throw new Error("This browser cannot create the program audio bus.");
  }

  return {
    audioContext,
    programDestination,
    programTrack,
    channels: {},
  };
}

function getOrCreateBroadcastMixState(room: Room): BroadcastMixState {
  const existing = broadcastMixStates.get(room);

  if (existing) {
    return existing;
  }

  const nextState = createBroadcastMixState();
  broadcastMixStates.set(room, nextState);
  return nextState;
}

async function createBroadcastAudioChannel(
  audioContext: AudioContext,
  rawTrack: MediaStreamTrack,
  initialGain: number,
): Promise<BroadcastAudioChannel> {
  if (audioContext.state === "suspended") {
    await audioContext.resume().catch(() => undefined);
  }

  const sourceNode = audioContext.createMediaStreamSource(new MediaStream([rawTrack]));
  const gainNode = audioContext.createGain();
  const destination = audioContext.createMediaStreamDestination();

  gainNode.gain.value = clampBroadcastGain(initialGain);
  sourceNode.connect(gainNode);
  gainNode.connect(destination);

  const [outputTrack] = destination.stream.getAudioTracks();

  if (!outputTrack) {
    sourceNode.disconnect();
    gainNode.disconnect();
    rawTrack.stop();
    throw new Error("Could not create a publishable audio mix for this source.");
  }

  return {
    gainNode,
    desiredGain: clampBroadcastGain(initialGain),
    enabled: true,
    outputTrack,
    rawTrack,
    sourceNode,
  };
}

function setProgramChannelEnabled(channel: BroadcastProgramChannel, enabled: boolean): void {
  channel.enabled = enabled;
  channel.gainNode.gain.value = enabled ? channel.desiredGain : 0;
}

function setProgramChannelVolume(channel: BroadcastProgramChannel, nextGain: number): void {
  channel.desiredGain = clampBroadcastGain(nextGain);
  channel.gainNode.gain.value = channel.enabled ? channel.desiredGain : 0;
}

function createProgramTrackChannel(
  audioContext: AudioContext,
  rawTrack: MediaStreamTrack,
  destination: MediaStreamAudioDestinationNode,
  initialGain: number,
): BroadcastProgramChannel {
  const sourceNode = audioContext.createMediaStreamSource(new MediaStream([rawTrack]));
  const gainNode = audioContext.createGain();
  const channel: BroadcastProgramChannel = {
    gainNode,
    desiredGain: clampBroadcastGain(initialGain),
    enabled: true,
    disconnect: () => {
      sourceNode.disconnect();
      gainNode.disconnect();
      rawTrack.stop();
    },
  };

  sourceNode.connect(gainNode);
  gainNode.connect(destination);
  setProgramChannelVolume(channel, initialGain);
  return channel;
}

type CaptureEnabledMediaElement = HTMLMediaElement & {
  captureStream?: () => MediaStream;
  mozCaptureStream?: () => MediaStream;
};

function captureMediaElementStream(element: HTMLMediaElement): MediaStream {
  const captureEnabledElement = element as CaptureEnabledMediaElement;
  const stream =
    captureEnabledElement.captureStream?.() ?? captureEnabledElement.mozCaptureStream?.();

  if (!stream) {
    throw new Error("This browser cannot route local file playback into the broadcast mixer.");
  }

  return stream;
}

function createProgramElementChannel(
  audioContext: AudioContext,
  element: HTMLMediaElement,
  destination: MediaStreamAudioDestinationNode,
  initialGain: number,
): BroadcastProgramChannel {
  const stream = captureMediaElementStream(element);
  const sourceNode = audioContext.createMediaStreamSource(stream);
  const gainNode = audioContext.createGain();
  const channel: BroadcastProgramChannel = {
    gainNode,
    desiredGain: clampBroadcastGain(initialGain),
    enabled: true,
    element,
    disconnect: () => {
      sourceNode.disconnect();
      gainNode.disconnect();
      stream.getTracks().forEach((track) => track.stop());
    },
  };

  sourceNode.connect(gainNode);
  gainNode.connect(destination);
  setProgramChannelVolume(channel, initialGain);
  return channel;
}

async function publishProgramTrack(room: Room): Promise<void> {
  const publication = room.localParticipant.getTrackPublication(Track.Source.ScreenShareAudio);

  if (publication?.track) {
    return;
  }

  const mixState = getOrCreateBroadcastMixState(room);
  await room.localParticipant.publishTrack(mixState.programTrack, {
    source: Track.Source.ScreenShareAudio,
    name: "program-audio",
  });
}

function attachProgramSource(
  room: Room,
  input: PreparedBroadcastInput,
  initialGain: number,
): void {
  const audioTrack = ensureDisplayAudioTrack(input.tracks, input.source);
  const mixState = getOrCreateBroadcastMixState(room);

  mixState.channels.programSource?.disconnect();
  mixState.channels.programSource = createProgramTrackChannel(
    mixState.audioContext,
    audioTrack.mediaStreamTrack,
    mixState.programDestination,
    initialGain,
  );
  broadcastCaptureTracks.set(room, input.tracks);
}

async function publishMicrophoneTrack(room: Room, initialGain = 1): Promise<void> {
  const existingPublication = room.localParticipant.getTrackPublication(Track.Source.Microphone);

  if (existingPublication?.track) {
    await existingPublication.track.unmute();
    setBroadcastInputVolume(room, "microphone", initialGain);
    return;
  }

  assertMicrophoneSupport();

  const stream = await navigator.mediaDevices.getUserMedia({
    audio: true,
  });
  const [rawTrack] = stream.getAudioTracks();

  if (!rawTrack) {
    throw new Error("No microphone audio was captured from this device.");
  }

  const mixState = getOrCreateBroadcastMixState(room);
  const channel = await createBroadcastAudioChannel(mixState.audioContext, rawTrack, initialGain);

  try {
    mixState.channels.microphone = channel;
    await room.localParticipant.publishTrack(channel.outputTrack, {
      source: Track.Source.Microphone,
      name: "microphone",
    });
  } catch (error) {
    delete mixState.channels.microphone;
    channel.sourceNode.disconnect();
    channel.gainNode.disconnect();
    channel.outputTrack.stop();
    channel.rawTrack.stop();
    throw error;
  }
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
  options: BroadcasterJoinOptions = {},
): Promise<Room> {
  const { micEnabled = true, micGain = 1, sourceGain = 1, deckGain = 1, deckElement = null } = options;
  const room = new Room();
  room.on(RoomEvent.Disconnected, () => {
    cleanupBroadcastCapture(room);
  });

  await room.connect(ensureLiveKitUrl(), token);

  try {
    await publishProgramTrack(room);

    if (micEnabled) {
      await publishMicrophoneTrack(room, micGain);
    }

    if (input) {
      attachProgramSource(room, input, sourceGain);
    }

    if (deckElement && deckElement.currentSrc) {
      bindMusicDeckElement(room, deckElement, deckGain);
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

export async function setMicEnabled(room: Room, enabled: boolean, initialGain = 1): Promise<void> {
  if (enabled) {
    await publishMicrophoneTrack(room, initialGain);
    return;
  }

  const publication = room.localParticipant.getTrackPublication(Track.Source.Microphone);
  const track = publication?.track;

  if (!track) {
    return;
  }

  await track.mute();
}

export async function setBroadcastInputEnabled(
  room: Room,
  source: BroadcastInput,
  enabled: boolean,
): Promise<void> {
  if (source === "microphone") {
    await setMicEnabled(room, enabled);
    return;
  }

  const channel = broadcastMixStates.get(room)?.channels.programSource;

  if (!channel) {
    return;
  }

  setProgramChannelEnabled(channel, enabled);
}

export function bindMusicDeckElement(
  room: Room,
  element: HTMLMediaElement,
  initialGain = 1,
): void {
  const mixState = getOrCreateBroadcastMixState(room);
  const existingChannel = mixState.channels.musicDeck;

  if (existingChannel?.element === element) {
    setProgramChannelVolume(existingChannel, initialGain);
    return;
  }

  existingChannel?.disconnect();
  mixState.channels.musicDeck = createProgramElementChannel(
    mixState.audioContext,
    element,
    mixState.programDestination,
    initialGain,
  );
}

export function setMusicDeckVolume(room: Room, nextVolume: number): void {
  const channel = broadcastMixStates.get(room)?.channels.musicDeck;

  if (!channel) {
    return;
  }

  setProgramChannelVolume(channel, nextVolume);
}

export function setBroadcastInputVolume(room: Room, source: BroadcastInput, nextVolume: number): void {
  const mixState = broadcastMixStates.get(room);

  if (!mixState) {
    return;
  }

  if (source === "microphone") {
    const channel = mixState.channels.microphone;

    if (!channel) {
      return;
    }

    channel.desiredGain = clampBroadcastGain(nextVolume);
    channel.gainNode.gain.value = channel.desiredGain;
    return;
  }

  const channel = mixState.channels.programSource;

  if (!channel) {
    return;
  }

  setProgramChannelVolume(channel, nextVolume);
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
