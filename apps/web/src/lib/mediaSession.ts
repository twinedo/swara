import { browser } from "$app/environment";

interface MediaSessionStation {
  title: string;
  subtitle?: string;
  artworkSrc?: string;
}

interface MediaSessionHandlers {
  onPlay?: () => void;
  onPause?: () => void;
  onStop?: () => void;
}

function hasMediaSession(): boolean {
  return browser && "mediaSession" in navigator;
}

export function setMediaSession(
  station: MediaSessionStation,
  handlers: MediaSessionHandlers = {},
): void {
  if (!hasMediaSession()) {
    return;
  }

  navigator.mediaSession.metadata = new MediaMetadata({
    title: station.title,
    artist: station.subtitle ?? "Swara",
    album: "Swara Live Radio",
    artwork: station.artworkSrc
      ? [
          {
            src: station.artworkSrc,
            sizes: "512x512",
            type: "image/svg+xml",
          },
        ]
      : [],
  });

  navigator.mediaSession.setActionHandler("play", handlers.onPlay ?? null);
  navigator.mediaSession.setActionHandler("pause", handlers.onPause ?? null);
  navigator.mediaSession.setActionHandler("stop", handlers.onStop ?? null);
  navigator.mediaSession.setActionHandler("seekbackward", null);
  navigator.mediaSession.setActionHandler("seekforward", null);
  navigator.mediaSession.setActionHandler("previoustrack", null);
  navigator.mediaSession.setActionHandler("nexttrack", null);
}

export function setPlaybackState(state: MediaSessionPlaybackState): void {
  if (!hasMediaSession()) {
    return;
  }

  navigator.mediaSession.playbackState = state;
}

export function clearMediaSession(): void {
  if (!hasMediaSession()) {
    return;
  }

  navigator.mediaSession.metadata = null;
  navigator.mediaSession.playbackState = "none";
}
