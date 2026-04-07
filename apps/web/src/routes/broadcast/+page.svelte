<script lang="ts">
  import { browser } from "$app/environment";
  import { onDestroy } from "svelte";
  import { get } from "svelte/store";

  import { sendBroadcastHeartbeat, startBroadcast, stopBroadcast } from "$lib/api/client";
  import FrequencyDisplay from "$lib/components/FrequencyDisplay.svelte";
  import Waveform from "$lib/components/Waveform.svelte";
  import {
    bindMusicDeckElement,
    type BroadcastInput,
    joinAsBroadcaster,
    leaveRoom,
    prepareBroadcastInput,
    releasePreparedBroadcastInput,
    setBroadcastInputEnabled,
    setBroadcastInputVolume,
    setMusicDeckVolume,
    setMicEnabled,
  } from "$lib/livekit";
  import {
    broadcastState,
    broadcasterRoom,
    isOnAir,
    ownedChannel,
    resetBroadcastState,
  } from "$lib/stores/channel";
  import { user } from "$lib/stores/user";

  const programAudioInputs: Array<{
    id: "none" | Exclude<BroadcastInput, "microphone">;
    label: string;
    detail: string;
  }> = [
    {
      id: "none",
      label: "No Program Audio",
      detail: "Voice-only broadcast with the microphone.",
    },
    {
      id: "tab-audio",
      label: "Tab Audio",
      detail: "Share a browser tab and enable audio.",
    },
    {
      id: "desktop-audio",
      label: "Desktop Audio",
      detail: "Share desktop audio when the browser offers it.",
    },
  ];

  const MAX_MUSIC_DECK_TRACKS = 10;

  type DeckTrack = {
    id: string;
    file: File;
    name: string;
    url: string;
    duration: number | null;
  };

  let programAudioInput: "none" | Exclude<BroadcastInput, "microphone"> = "none";
  let micEnabled = true;
  let micMuted = false;
  let micVolume = 100;
  let sourceMuted = false;
  let sourceVolume = 100;
  let deckVolume = 100;
  let musicDeckPicker: HTMLInputElement | null = null;
  let musicDeckElement: HTMLAudioElement | null = null;
  let musicDeckTracks: DeckTrack[] = [];
  let activeDeckIndex = -1;
  let activeDeckTrack: DeckTrack | null = null;
  let deckCurrentTime = 0;
  let deckDuration = 0;
  let deckPlaying = false;
  let deckPendingAutoplay = false;
  let statusMessage: string | null = null;
  let heartbeatId: number | null = null;

  function canUseDisplayAudio(): boolean {
    return browser && window.isSecureContext && !!navigator.mediaDevices?.getDisplayMedia;
  }

  function hasMusicDeckTracks(): boolean {
    return musicDeckTracks.length > 0;
  }

  function createDeckTrackId(index: number): string {
    if (browser && "crypto" in window && "randomUUID" in window.crypto) {
      return window.crypto.randomUUID();
    }

    return `deck-${Date.now()}-${index}`;
  }

  function formatDeckTime(totalSeconds: number): string {
    if (!Number.isFinite(totalSeconds) || totalSeconds < 0) {
      return "0:00";
    }

    const minutes = Math.floor(totalSeconds / 60);
    const seconds = Math.floor(totalSeconds % 60);
    return `${minutes}:${seconds.toString().padStart(2, "0")}`;
  }

  function syncActiveDeckTrack(autoplay = deckPlaying): void {
    if (!musicDeckElement) {
      return;
    }

    if (!activeDeckTrack) {
      deckPendingAutoplay = false;
      deckPlaying = false;
      deckCurrentTime = 0;
      deckDuration = 0;
      musicDeckElement.pause();
      musicDeckElement.removeAttribute("src");
      musicDeckElement.load();
      return;
    }

    if (musicDeckElement.currentSrc !== activeDeckTrack.url) {
      deckPendingAutoplay = autoplay;
      deckPlaying = autoplay;
      deckCurrentTime = 0;
      deckDuration = activeDeckTrack.duration ?? 0;
      musicDeckElement.pause();
      musicDeckElement.src = activeDeckTrack.url;
      musicDeckElement.load();
      return;
    }

    if (!autoplay) {
      return;
    }

    deckPendingAutoplay = false;
    void musicDeckElement.play().catch((error) => {
      deckPlaying = false;
      statusMessage = error instanceof Error ? error.message : "Could not start the music deck.";
    });
  }

  function openMusicDeckPicker() {
    musicDeckPicker?.click();
  }

  function handleMusicDeckSelection(event: Event) {
    const target = event.currentTarget as HTMLInputElement | null;
    const selectedFiles = Array.from(target?.files ?? []);

    if (!selectedFiles.length) {
      return;
    }

    const remainingSlots = MAX_MUSIC_DECK_TRACKS - musicDeckTracks.length;
    const audioFiles = selectedFiles.filter((file) => file.type.startsWith("audio/"));
    const acceptedFiles = audioFiles.slice(0, Math.max(0, remainingSlots));

    if (!acceptedFiles.length) {
      statusMessage =
        remainingSlots <= 0
          ? `Music deck is full. Remove a track before adding more than ${MAX_MUSIC_DECK_TRACKS}.`
          : "Select audio files only for the music deck.";
      if (target) {
        target.value = "";
      }
      return;
    }

    musicDeckTracks = [
      ...musicDeckTracks,
      ...acceptedFiles.map((file, index) => ({
        id: createDeckTrackId(index),
        file,
        name: file.name,
        url: URL.createObjectURL(file),
        duration: null,
      })),
    ];

    if (activeDeckIndex === -1) {
      activeDeckIndex = 0;
      syncActiveDeckTrack(false);
    }

    if (acceptedFiles.length !== selectedFiles.length) {
      statusMessage = `Loaded ${acceptedFiles.length} audio file(s). Non-audio files or tracks above the ${MAX_MUSIC_DECK_TRACKS}-file limit were skipped.`;
    } else {
      statusMessage = null;
    }

    if (target) {
      target.value = "";
    }
  }

  async function playDeck() {
    if (!activeDeckTrack) {
      statusMessage = "Add at least one music file before starting the deck.";
      return;
    }

    if (!musicDeckElement) {
      return;
    }

    if (musicDeckElement.currentSrc !== activeDeckTrack.url) {
      syncActiveDeckTrack(true);
      return;
    }

    try {
      await musicDeckElement.play();
    } catch (error) {
      deckPlaying = false;
      statusMessage = error instanceof Error ? error.message : "Could not start the music deck.";
    }
  }

  function pauseDeck() {
    musicDeckElement?.pause();
  }

  function stopDeck() {
    if (!musicDeckElement) {
      return;
    }

    musicDeckElement.pause();
    musicDeckElement.currentTime = 0;
    deckCurrentTime = 0;
    deckPlaying = false;
    deckPendingAutoplay = false;
  }

  function selectDeckTrack(index: number, autoplay = deckPlaying) {
    if (index < 0 || index >= musicDeckTracks.length) {
      return;
    }

    activeDeckIndex = index;
    syncActiveDeckTrack(autoplay);
  }

  function playPreviousDeckTrack(autoplay = deckPlaying) {
    if (activeDeckIndex <= 0) {
      return;
    }

    selectDeckTrack(activeDeckIndex - 1, autoplay);
  }

  function playNextDeckTrack(autoplay = deckPlaying) {
    if (activeDeckIndex < 0 || activeDeckIndex >= musicDeckTracks.length - 1) {
      return;
    }

    selectDeckTrack(activeDeckIndex + 1, autoplay);
  }

  function removeDeckTrack(index: number) {
    const targetTrack = musicDeckTracks[index];

    if (!targetTrack) {
      return;
    }

    const wasActive = index === activeDeckIndex;
    const shouldAutoplay = wasActive && deckPlaying;
    URL.revokeObjectURL(targetTrack.url);
    musicDeckTracks = musicDeckTracks.filter((_, trackIndex) => trackIndex !== index);

    if (!musicDeckTracks.length) {
      activeDeckIndex = -1;
      syncActiveDeckTrack(false);
      return;
    }

    if (index < activeDeckIndex) {
      activeDeckIndex -= 1;
      return;
    }

    if (wasActive) {
      activeDeckIndex = Math.min(index, musicDeckTracks.length - 1);
      syncActiveDeckTrack(shouldAutoplay);
    }
  }

  function handleDeckCanPlay() {
    if (deckPendingAutoplay) {
      void playDeck();
    }
  }

  function handleDeckLoadedMetadata() {
    if (!musicDeckElement || activeDeckIndex < 0) {
      return;
    }

    const nextDuration = Number.isFinite(musicDeckElement.duration) ? musicDeckElement.duration : 0;
    deckDuration = nextDuration;
    musicDeckTracks = musicDeckTracks.map((track, index) =>
      index === activeDeckIndex ? { ...track, duration: nextDuration } : track,
    );
  }

  function handleDeckTimeUpdate() {
    if (!musicDeckElement) {
      return;
    }

    deckCurrentTime = musicDeckElement.currentTime;
  }

  function handleDeckSeekInput(event: Event) {
    const target = event.currentTarget as HTMLInputElement | null;

    if (!target || !musicDeckElement) {
      return;
    }

    const nextTime = Number(target.value);
    musicDeckElement.currentTime = nextTime;
    deckCurrentTime = nextTime;
  }

  function handleDeckPlay() {
    deckPlaying = true;
    deckPendingAutoplay = false;
  }

  function handleDeckPause() {
    deckPlaying = false;
  }

  function handleDeckEnded() {
    if (activeDeckIndex >= 0 && activeDeckIndex < musicDeckTracks.length - 1) {
      playNextDeckTrack(true);
      return;
    }

    stopDeck();
  }

  function stopHeartbeatLoop() {
    if (heartbeatId != null) {
      window.clearInterval(heartbeatId);
      heartbeatId = null;
    }
  }

  function ensureHeartbeatLoop() {
    if (heartbeatId != null || typeof window === "undefined" || !$ownedChannel) {
      return;
    }

    heartbeatId = window.setInterval(async () => {
      if (!$ownedChannel) {
        return;
      }

      try {
        await sendBroadcastHeartbeat($ownedChannel.id);
      } catch (error) {
        broadcastState.set("interrupted");
        statusMessage =
          error instanceof Error
            ? error.message
            : "The broadcast heartbeat failed and the room should be reconnected.";
        stopHeartbeatLoop();
        await leaveRoom(get(broadcasterRoom));
      }
    }, 15_000);
  }

  async function goLive() {
    if (!$ownedChannel) {
      return;
    }

    statusMessage = null;
    micMuted = !micEnabled;
    sourceMuted = false;
    broadcastState.set("connecting");

    let preparedInput = null;

    try {
      if (programAudioInput !== "none") {
        preparedInput = await prepareBroadcastInput(programAudioInput);
      }
    } catch (error) {
      broadcastState.set("error");
      statusMessage =
        error instanceof Error ? error.message : "This device cannot access the selected audio source.";
      return;
    }

    if (!micEnabled && !preparedInput && !hasMusicDeckTracks()) {
      broadcastState.set("error");
      statusMessage = "Turn on the microphone, choose tab/desktop audio, or load music deck tracks before going live.";
      return;
    }

    let broadcastStarted = false;

    try {
      const session = await startBroadcast($ownedChannel.id);
      broadcastStarted = true;
      const room = await joinAsBroadcaster(session.livekitToken, preparedInput, {
        micEnabled,
        micGain: micVolume / 100,
        sourceGain: sourceVolume / 100,
        deckGain: deckVolume / 100,
        deckElement: hasMusicDeckTracks() ? musicDeckElement : null,
      });
      broadcasterRoom.set(room);
      broadcastState.set("live");
      ensureHeartbeatLoop();
    } catch (error) {
      releasePreparedBroadcastInput(preparedInput);

      if (broadcastStarted) {
        await stopBroadcast($ownedChannel.id).catch(() => undefined);
      }

      broadcastState.set("error");
      statusMessage =
        error instanceof Error ? error.message : "Could not start broadcasting.";
    }
  }

  async function endBroadcast() {
    if (!$ownedChannel) {
      return;
    }

    statusMessage = null;
    broadcastState.set("stopping");
    stopHeartbeatLoop();

    try {
      await leaveRoom(get(broadcasterRoom));
      await stopBroadcast($ownedChannel.id);
      resetBroadcastState();
      stopDeck();
      micMuted = false;
      sourceMuted = false;
    } catch (error) {
      broadcastState.set("error");
      statusMessage =
        error instanceof Error ? error.message : "Could not stop broadcasting cleanly.";
    }
  }

  async function toggleMic() {
    if (!$broadcasterRoom) {
      return;
    }

    const nextEnabled = micMuted;
    await setMicEnabled($broadcasterRoom, nextEnabled, micVolume / 100);
    micMuted = !micMuted;
  }

  async function toggleProgramAudio() {
    if (!$broadcasterRoom || programAudioInput === "none") {
      return;
    }

    const nextEnabled = sourceMuted;
    await setBroadcastInputEnabled($broadcasterRoom, programAudioInput, nextEnabled);
    sourceMuted = !sourceMuted;
  }

  function handleMicVolumeInput(event: Event) {
    const target = event.currentTarget as HTMLInputElement | null;

    if (!target) {
      return;
    }

    micVolume = Number(target.value);

    if ($broadcasterRoom) {
      setBroadcastInputVolume($broadcasterRoom, "microphone", micVolume / 100);
    }
  }

  function handleSourceVolumeInput(event: Event) {
    const target = event.currentTarget as HTMLInputElement | null;

    if (!target) {
      return;
    }

    sourceVolume = Number(target.value);

    if ($broadcasterRoom && programAudioInput !== "none") {
      setBroadcastInputVolume($broadcasterRoom, programAudioInput, sourceVolume / 100);
    }
  }

  function handleDeckVolumeInput(event: Event) {
    const target = event.currentTarget as HTMLInputElement | null;

    if (!target) {
      return;
    }

    deckVolume = Number(target.value);

    if ($broadcasterRoom) {
      setMusicDeckVolume($broadcasterRoom, deckVolume / 100);
    }
  }

  $: if (!canUseDisplayAudio() && programAudioInput !== "none") {
    programAudioInput = "none";
  }

  $: activeDeckTrack = musicDeckTracks[activeDeckIndex] ?? null;
  $: micToggleLabel = micMuted ? "Unmute Mic" : "Mute Mic";
  $: sourceToggleLabel = sourceMuted ? "Unmute Source" : "Mute Source";

  $: if (musicDeckElement && activeDeckTrack && musicDeckElement.currentSrc !== activeDeckTrack.url) {
    syncActiveDeckTrack(deckPendingAutoplay || deckPlaying);
  }

  $: if (musicDeckElement && !activeDeckTrack && musicDeckElement.currentSrc) {
    syncActiveDeckTrack(false);
  }

  $: if ($broadcasterRoom && musicDeckElement && activeDeckTrack) {
    try {
      bindMusicDeckElement($broadcasterRoom, musicDeckElement, deckVolume / 100);
    } catch (error) {
      statusMessage =
        error instanceof Error ? error.message : "Could not route the music deck into the broadcast mixer.";
    }
  }

  $: if ($broadcastState === "live") {
    ensureHeartbeatLoop();
  } else {
    stopHeartbeatLoop();
  }

  onDestroy(() => {
    stopHeartbeatLoop();
    musicDeckTracks.forEach((track) => {
      URL.revokeObjectURL(track.url);
    });
  });
</script>

<div class:guest-layout={!$user} class="page-shell">
  {#if !$user}
    <section class="gate panel">
      <span class="section-label">Broadcast</span>
      <h1>Sign in required</h1>
      <p class="status-copy">
        The broadcaster controls are available after authentication because the API checks channel
        ownership before issuing a publisher token.
      </p>
      <a class="ghost-button link-button" href="/settings">Open Settings</a>
    </section>
  {:else if !$ownedChannel}
    <section class="gate panel">
      <span class="section-label">Broadcast</span>
      <h1>No owned channel</h1>
      <p class="status-copy">
        Create your station first in Settings, then this view can start and end a real broadcast.
      </p>
      <a class="ghost-button link-button" href="/settings">Create Channel</a>
    </section>
  {:else}
    <div class="broadcast-shell">
      <section class="hero panel">
        <div class="hero-head">
          <span class="section-label">Broadcast Console</span>
          {#if $isOnAir}
            <span class="live-pill">On Air</span>
          {/if}
        </div>

        <FrequencyDisplay frequency={$ownedChannel.frequency} size="desktop" />

        <div class="station-block">
          <strong>{$ownedChannel.name}</strong>
          <p>Owned by @{$user.username}</p>
          <Waveform active={$broadcastState === "live"} bars={7} />
        </div>

        {#if !$isOnAir}
          <div class="source-picker">
            <span class="section-label">Microphone</span>
            <button
              type="button"
              class:active-source={micEnabled}
              class="ghost-button source-button mic-button"
              on:click={() => (micEnabled = !micEnabled)}
            >
              <strong>{micEnabled ? "Mic On Air" : "Mic Off Air"}</strong>
              <span>
                {micEnabled
                  ? "Your voice stays live and can speak over program audio."
                  : "Keep the mic muted and publish only the selected program audio."}
              </span>
            </button>

            <span class="section-label">Program Audio</span>
            <div class="source-grid">
              {#each programAudioInputs as input}
                <button
                  type="button"
                  class:active-source={programAudioInput === input.id}
                  class="ghost-button source-button"
                  disabled={(input.id !== "none" && !canUseDisplayAudio()) || $broadcastState === "connecting"}
                  on:click={() => (programAudioInput = input.id)}
                >
                  <strong>{input.label}</strong>
                  <span>{input.detail}</span>
                </button>
              {/each}
            </div>
            {#if !canUseDisplayAudio()}
              <p class="status-copy">
                Tab and desktop audio capture require a secure desktop browser context with screen-sharing support.
              </p>
            {/if}
          </div>
        {/if}

        <div class="mixer-panel">
          <span class="section-label">Mixer</span>
          <div class="mixer-grid">
            <div class="mix-strip">
              <div class="mix-head">
                <strong>Mic Level</strong>
                <span>{micVolume}%</span>
              </div>
              <input
                aria-label="Microphone volume"
                class="mix-slider"
                type="range"
                min="0"
                max="200"
                step="5"
                value={micVolume}
                on:input={handleMicVolumeInput}
              />
              <p>
                Keep voice present over the program feed. `100%` is normal level and `200%`
                boosts harder.
              </p>
            </div>

            {#if programAudioInput !== "none"}
              <div class="mix-strip">
                <div class="mix-head">
                  <strong>{programAudioInput === "tab-audio" ? "Tab Level" : "Desktop Level"}</strong>
                  <span>{sourceVolume}%</span>
                </div>
                <input
                  aria-label="Program audio volume"
                  class="mix-slider"
                  type="range"
                  min="0"
                  max="200"
                  step="5"
                  value={sourceVolume}
                  on:input={handleSourceVolumeInput}
                />
                <p>
                  Shape the shared tab or desktop source without changing the music deck or mic.
                </p>
              </div>
            {/if}

            {#if musicDeckTracks.length}
              <div class="mix-strip">
                <div class="mix-head">
                  <strong>Deck Level</strong>
                  <span>{deckVolume}%</span>
                </div>
                <input
                  aria-label="Music deck volume"
                  class="mix-slider"
                  type="range"
                  min="0"
                  max="200"
                  step="5"
                  value={deckVolume}
                  on:input={handleDeckVolumeInput}
                />
                <p>
                  Ride local music files as their own program source and balance them against the
                  live mic.
                </p>
              </div>
            {/if}
          </div>
        </div>

        <div class="controls">
          {#if $isOnAir}
            <button class="ghost-button" type="button" on:click={toggleMic}>
              {micToggleLabel}
            </button>
            {#if programAudioInput !== "none"}
              <button class="ghost-button" type="button" on:click={toggleProgramAudio}>
                {sourceToggleLabel}
              </button>
            {/if}
            <button class="danger-button" type="button" on:click={endBroadcast}>
              End Broadcast
            </button>
          {:else}
            <button
              class="primary-button"
              type="button"
              disabled={$broadcastState === "connecting" || (!micEnabled && programAudioInput === "none" && !hasMusicDeckTracks())}
              on:click={goLive}
            >
              {$broadcastState === "connecting" ? "Connecting..." : "Go Live"}
            </button>
          {/if}
        </div>

        {#if statusMessage}
          <p class="status-copy">{statusMessage}</p>
        {/if}
      </section>

      <section class="status-card panel music-deck-card">
        <div class="deck-head">
          <span class="section-label">Music Deck</span>
          <span class="mono">{musicDeckTracks.length}/{MAX_MUSIC_DECK_TRACKS} loaded</span>
        </div>

        <input
          bind:this={musicDeckPicker}
          accept="audio/*"
          class="deck-file-input"
          multiple
          type="file"
          on:change={handleMusicDeckSelection}
        />

        <div class="deck-toolbar">
          <button class="ghost-button deck-console-button deck-add-button" type="button" on:click={openMusicDeckPicker}>
            Add Music
          </button>

          {#if activeDeckTrack}
            <div class="deck-queue-badge mono">
              Track {activeDeckIndex + 1} of {musicDeckTracks.length}
            </div>
          {/if}
        </div>

        {#if activeDeckTrack}
          <div class="deck-player-shell">
            <div class="deck-player-header">
              <div class="deck-player-copy">
                <span class="deck-player-label">Now Playing</span>
                <strong>{activeDeckTrack.name}</strong>
              </div>
              <div class="deck-player-meter" aria-hidden="true">
                <span></span>
                <span></span>
                <span></span>
                <span></span>
                <span></span>
                <span></span>
              </div>
            </div>

            <div class="deck-progress">
              <input
                aria-label="Seek music deck track"
                class="deck-progress-slider"
                max={deckDuration || 0}
                min="0"
                step="0.1"
                type="range"
                value={Math.min(deckCurrentTime, deckDuration || 0)}
                on:input={handleDeckSeekInput}
              />
              <div class="deck-time-row mono">
                <span>{formatDeckTime(deckCurrentTime)}</span>
                <span>{formatDeckTime(deckDuration)}</span>
              </div>
            </div>

            <div class="deck-transport-cluster">
              <div class="deck-transport-group deck-transport-group-seek">
                <button
                  aria-label="Previous track"
                  class="ghost-button deck-console-button deck-console-button-icon"
                  type="button"
                  disabled={activeDeckIndex <= 0}
                  on:click={() => playPreviousDeckTrack(deckPlaying)}
                >
                  <svg aria-hidden="true" class="deck-icon" viewBox="0 0 24 24">
                    <path
                      d="M17 6L9 12L17 18"
                      fill="none"
                      stroke="currentColor"
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="1.8"
                    ></path>
                    <path
                      d="M10 6L2 12L10 18"
                      fill="none"
                      stroke="currentColor"
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="1.8"
                    ></path>
                  </svg>
                  <span class="sr-only">Previous</span>
                </button>
                <button
                  aria-label={deckPlaying ? "Pause deck" : "Play deck"}
                  class="ghost-button deck-console-button deck-console-button-icon deck-console-button-primary deck-console-button-play"
                  type="button"
                  on:click={deckPlaying ? pauseDeck : playDeck}
                >
                  {#if deckPlaying}
                    <svg aria-hidden="true" class="deck-icon deck-icon-play" viewBox="0 0 24 24">
                      <path d="M8 6H11V18H8zM13 6H16V18H13z" fill="currentColor"></path>
                    </svg>
                  {:else}
                    <svg aria-hidden="true" class="deck-icon deck-icon-play" viewBox="0 0 24 24">
                      <path d="M8 6L18 12L8 18V6z" fill="currentColor"></path>
                    </svg>
                  {/if}
                  <span class="sr-only">{deckPlaying ? "Pause Deck" : "Play Deck"}</span>
                </button>
                <button
                  aria-label="Next track"
                  class="ghost-button deck-console-button deck-console-button-icon"
                  type="button"
                  disabled={activeDeckIndex >= musicDeckTracks.length - 1}
                  on:click={() => playNextDeckTrack(deckPlaying)}
                >
                  <svg aria-hidden="true" class="deck-icon" viewBox="0 0 24 24">
                    <path
                      d="M7 6L15 12L7 18"
                      fill="none"
                      stroke="currentColor"
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="1.8"
                    ></path>
                    <path
                      d="M14 6L22 12L14 18"
                      fill="none"
                      stroke="currentColor"
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="1.8"
                    ></path>
                  </svg>
                  <span class="sr-only">Next</span>
                </button>
              </div>

              <button class="ghost-button deck-console-button deck-stop-button" type="button" on:click={stopDeck}>
                <svg aria-hidden="true" class="deck-icon" viewBox="0 0 24 24">
                  <path d="M7 7H17V17H7z" fill="currentColor"></path>
                </svg>
                <span>Stop</span>
              </button>
            </div>
          </div>
        {:else}
          <div class="deck-empty-state">
            <span class="deck-player-label">Music Deck Ready</span>
            <strong>Load tracks to build a live queue.</strong>
            <p class="status-copy">
              Add up to {MAX_MUSIC_DECK_TRACKS} local audio files. They stay in this browser session only.
            </p>
          </div>
        {/if}

        {#if musicDeckTracks.length}
          <div class="deck-list">
            {#each musicDeckTracks as track, index}
              <div class:active-deck-track={index === activeDeckIndex} class="deck-row">
                <button class="deck-track-button" type="button" on:click={() => selectDeckTrack(index, deckPlaying)}>
                  <strong>{track.name}</strong>
                  <span>
                    {track.duration != null ? formatDeckTime(track.duration) : "Ready to load"}
                  </span>
                </button>
                <button class="ghost-button mini-button" type="button" on:click={() => removeDeckTrack(index)}>
                  Remove
                </button>
              </div>
            {/each}
          </div>
        {/if}

        <audio
          bind:this={musicDeckElement}
          class="deck-audio"
          preload="metadata"
          on:canplay={handleDeckCanPlay}
          on:ended={handleDeckEnded}
          on:loadedmetadata={handleDeckLoadedMetadata}
          on:pause={handleDeckPause}
          on:play={handleDeckPlay}
          on:timeupdate={handleDeckTimeUpdate}
        ></audio>
      </section>
    </div>
  {/if}
</div>

<style>
  .page-shell {
    padding: 18px 16px 0;
  }

  .gate,
  .hero,
  .status-card {
    padding: 20px;
  }

  .gate h1 {
    margin: 10px 0 12px;
    font-family: "Bebas Neue", sans-serif;
    font-size: 40px;
    letter-spacing: 2px;
  }

  .link-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    margin-top: 18px;
    text-decoration: none;
  }

  .broadcast-shell {
    display: grid;
    gap: 18px;
  }

  .hero-head,
  .controls {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
    justify-content: space-between;
  }

  .station-block {
    display: grid;
    gap: 8px;
    margin: 16px 0 20px;
  }

  .source-picker {
    display: grid;
    gap: 12px;
    margin-bottom: 20px;
  }

  .source-grid {
    display: grid;
    gap: 10px;
  }

  .music-deck-card {
    display: grid;
    gap: 14px;
    align-content: start;
  }

  .deck-head,
  .deck-toolbar {
    display: flex;
    align-items: center;
    gap: 12px;
    justify-content: space-between;
    flex-wrap: wrap;
  }

  .deck-toolbar {
    align-items: flex-start;
  }

  .deck-queue-badge {
    padding: 8px 12px;
    border: 1px solid rgba(232, 200, 74, 0.18);
    border-radius: 999px;
    background: rgba(232, 200, 74, 0.07);
    color: #f1e3a0;
    font-size: 11px;
    letter-spacing: 1.4px;
    text-transform: uppercase;
  }

  .deck-player-shell,
  .deck-empty-state {
    display: grid;
    gap: 14px;
    padding: 16px;
    border: 1px solid var(--border-subtle);
    border-radius: 22px;
    background:
      radial-gradient(circle at top left, rgba(232, 200, 74, 0.09), transparent 35%),
      linear-gradient(180deg, rgba(255, 255, 255, 0.035), rgba(255, 255, 255, 0.015));
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.04),
      0 16px 40px rgba(0, 0, 0, 0.18);
  }

  .deck-player-header {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 14px;
    align-items: start;
  }

  .deck-player-copy {
    display: grid;
    gap: 6px;
  }

  .deck-player-copy strong,
  .deck-empty-state strong {
    font-size: 18px;
    line-height: 1.25;
  }

  .deck-player-copy > span:last-child,
  .deck-empty-state .status-copy {
    color: var(--text-muted);
    line-height: 1.5;
    margin: 0;
  }

  .deck-player-label {
    font-family: "Share Tech Mono", monospace;
    font-size: 11px;
    letter-spacing: 1.8px;
    text-transform: uppercase;
    color: #f1e3a0;
  }

  .deck-player-meter {
    display: flex;
    align-items: end;
    gap: 4px;
    min-height: 52px;
    padding-top: 6px;
  }

  .deck-player-meter span {
    width: 5px;
    border-radius: 999px;
    background: linear-gradient(180deg, rgba(232, 200, 74, 0.92), rgba(232, 200, 74, 0.18));
    opacity: 0.88;
  }

  .deck-player-meter span:nth-child(1) { height: 18px; }
  .deck-player-meter span:nth-child(2) { height: 30px; }
  .deck-player-meter span:nth-child(3) { height: 22px; }
  .deck-player-meter span:nth-child(4) { height: 38px; }
  .deck-player-meter span:nth-child(5) { height: 26px; }
  .deck-player-meter span:nth-child(6) { height: 16px; }

  .deck-progress {
    display: grid;
    gap: 8px;
  }

  .deck-progress-slider {
    width: 100%;
    accent-color: #e8c84a;
  }

  .deck-time-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    color: var(--text-muted);
    font-size: 12px;
  }

  .deck-transport-cluster {
    display: grid;
    gap: 12px;
    align-items: center;
  }

  .deck-transport-group {
    display: grid;
    gap: 10px;
  }

  .deck-transport-group-seek {
    grid-template-columns: repeat(3, minmax(0, 1fr));
    align-items: center;
  }

  .deck-console-button {
    min-height: 56px;
    border-radius: 999px;
    padding: 0 20px;
    font-family: "Share Tech Mono", monospace;
    font-size: 11px;
    letter-spacing: 2px;
    text-transform: uppercase;
    background:
      radial-gradient(circle at top, rgba(255, 255, 255, 0.06), transparent 62%),
      rgba(255, 255, 255, 0.03);
    border-color: rgba(255, 255, 255, 0.08);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.04),
      0 10px 24px rgba(0, 0, 0, 0.22);
  }

  .deck-console-button:hover:enabled {
    border-color: rgba(232, 200, 74, 0.3);
    background:
      radial-gradient(circle at top, rgba(232, 200, 74, 0.1), transparent 62%),
      rgba(255, 255, 255, 0.05);
  }

  .deck-console-button-primary {
    border-color: rgba(232, 200, 74, 0.36);
    background:
      radial-gradient(circle at top, rgba(232, 200, 74, 0.15), transparent 65%),
      rgba(232, 200, 74, 0.06);
    color: #f7edbe;
  }

  .deck-console-button-icon {
    min-width: 56px;
    padding: 0;
    display: grid;
    place-items: center;
  }

  .deck-console-button-play {
    min-height: 68px;
    min-width: 68px;
  }

  .deck-icon {
    width: 20px;
    height: 20px;
  }

  .deck-icon-play {
    width: 24px;
    height: 24px;
  }

  .deck-stop-button {
    justify-self: start;
    min-width: 112px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
  }

  .deck-add-button {
    justify-self: start;
    min-width: 140px;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  .deck-file-input,
  .deck-audio {
    display: none;
  }

  .deck-list {
    display: grid;
    gap: 10px;
  }

  .deck-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 10px;
    align-items: center;
  }

  .deck-track-button {
    width: 100%;
    min-height: 0;
    border: 1px solid var(--border-subtle);
    border-radius: 16px;
    padding: 12px 14px;
    display: grid;
    gap: 6px;
    text-align: left;
    background: rgba(255, 255, 255, 0.02);
    color: inherit;
    cursor: pointer;
  }

  .deck-track-button strong {
    font-family: "Share Tech Mono", monospace;
    font-size: 11px;
    letter-spacing: 1.6px;
    text-transform: uppercase;
  }

  .deck-track-button span {
    color: var(--text-muted);
    line-height: 1.4;
  }

  .deck-row.active-deck-track .deck-track-button {
    border-color: rgba(232, 200, 74, 0.45);
    background: rgba(232, 200, 74, 0.08);
  }

  .mini-button {
    min-height: 0;
    padding: 10px 14px;
  }

  .mixer-panel {
    display: grid;
    gap: 12px;
    margin-bottom: 20px;
  }

  .mixer-grid {
    display: grid;
    gap: 12px;
  }

  .mix-strip {
    border: 1px solid var(--border-subtle);
    border-radius: 18px;
    padding: 14px 16px;
    background: rgba(255, 255, 255, 0.02);
    display: grid;
    gap: 10px;
  }

  .mix-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .mix-head strong,
  .mix-head span {
    font-family: "Share Tech Mono", monospace;
    font-size: 11px;
    letter-spacing: 1.6px;
    text-transform: uppercase;
  }

  .mix-head span {
    color: var(--text-muted);
  }

  .mix-slider {
    width: 100%;
    accent-color: #e8c84a;
  }

  .mix-strip p {
    margin: 0;
    color: var(--text-muted);
    line-height: 1.45;
  }

  .source-button {
    min-height: 0;
    border-radius: 16px;
    padding: 14px 16px;
    display: grid;
    gap: 6px;
    text-align: left;
    text-transform: none;
    letter-spacing: 0;
    font-family: "DM Sans", sans-serif;
  }

  .source-button strong {
    font-family: "Share Tech Mono", monospace;
    font-size: 11px;
    letter-spacing: 1.6px;
    text-transform: uppercase;
  }

  .source-button span {
    color: var(--text-muted);
    line-height: 1.45;
  }

  .source-button.active-source {
    border-color: rgba(232, 200, 74, 0.45);
    background: rgba(232, 200, 74, 0.08);
  }

  .station-block strong {
    font-size: 18px;
  }

  .station-block p {
    margin: 0;
    color: var(--text-muted);
  }

  @media (min-width: 1024px) {
    .page-shell {
      padding: 24px;
    }

    .page-shell.guest-layout {
      min-height: calc(100vh - 56px - var(--sat) - 28px);
      display: grid;
      place-items: center;
    }

    .page-shell.guest-layout > .gate {
      width: min(100%, 680px);
    }

    .broadcast-shell {
      grid-template-columns: minmax(0, 1.25fr) minmax(300px, 0.75fr);
    }

    .source-grid {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }

    .mixer-grid {
      grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    }

    .deck-transport-cluster {
      grid-template-columns: minmax(0, 1fr) auto;
    }

    .deck-add-button {
      align-self: start;
    }
  }
</style>
