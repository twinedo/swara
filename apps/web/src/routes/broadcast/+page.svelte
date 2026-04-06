<script lang="ts">
  import { browser } from "$app/environment";
  import { onDestroy } from "svelte";
  import { get } from "svelte/store";

  import { sendBroadcastHeartbeat, startBroadcast, stopBroadcast } from "$lib/api/client";
  import FrequencyDisplay from "$lib/components/FrequencyDisplay.svelte";
  import Waveform from "$lib/components/Waveform.svelte";
  import {
    type BroadcastInput,
    joinAsBroadcaster,
    leaveRoom,
    prepareBroadcastInput,
    releasePreparedBroadcastInput,
    setBroadcastInputEnabled,
    setBroadcastInputVolume,
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

  let programAudioInput: "none" | Exclude<BroadcastInput, "microphone"> = "none";
  let micEnabled = true;
  let micMuted = false;
  let micVolume = 100;
  let sourceMuted = false;
  let sourceVolume = 100;
  let statusMessage: string | null = null;
  let heartbeatId: number | null = null;

  function canUseDisplayAudio(): boolean {
    return browser && window.isSecureContext && !!navigator.mediaDevices?.getDisplayMedia;
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

    if (!micEnabled && !preparedInput) {
      broadcastState.set("error");
      statusMessage = "Turn on the microphone or choose tab/desktop audio before going live.";
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

  $: if (!canUseDisplayAudio() && programAudioInput !== "none") {
    programAudioInput = "none";
  }

  $: micToggleLabel = micMuted ? "Unmute Mic" : "Mute Mic";
  $: sourceToggleLabel = sourceMuted ? "Unmute Source" : "Mute Source";
  $: broadcastStepTwo =
    programAudioInput === "none"
      ? micEnabled
        ? "Join the LiveKit room and publish the local microphone."
        : "Choose at least one input before going live."
      : programAudioInput === "tab-audio"
        ? micEnabled
          ? "Join the LiveKit room, keep the mic live, and publish the shared browser tab audio."
          : "Choose a browser tab and enable audio sharing, then publish that tab audio."
        : micEnabled
          ? "Join the LiveKit room, keep the mic live, and publish the shared desktop audio."
          : "Choose a screen, window, or tab with audio, then publish the shared desktop audio.";

  $: if ($broadcastState === "live") {
    ensureHeartbeatLoop();
  } else {
    stopHeartbeatLoop();
  }

  onDestroy(() => {
    stopHeartbeatLoop();
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
                  Shape the shared source independently so listeners hear music, tabs, or desktop
                  audio at the right level.
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
              disabled={$broadcastState === "connecting" || (!micEnabled && programAudioInput === "none")}
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

      <section class="status-card panel">
        <span class="section-label">Flow</span>
        <div class="flow-row">
          <span class="mono">1</span>
          <p>Request publisher token from `/api/broadcast/start`.</p>
        </div>
        <div class="flow-row">
          <span class="mono">2</span>
          <p>{broadcastStepTwo}</p>
        </div>
        <div class="flow-row">
          <span class="mono">3</span>
          <p>Send a heartbeat every 15 seconds so dead-air detection does not flip the channel off.</p>
        </div>
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

  .flow-row {
    display: grid;
    grid-template-columns: 28px 1fr;
    gap: 12px;
    padding: 12px 0;
    border-bottom: 1px solid var(--border-subtle);
  }

  .flow-row:last-child {
    border-bottom: 0;
  }

  .flow-row p {
    margin: 0;
    color: var(--text-muted);
    line-height: 1.5;
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
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
