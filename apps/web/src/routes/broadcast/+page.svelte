<script lang="ts">
  import { onDestroy } from "svelte";
  import { get } from "svelte/store";

  import { sendBroadcastHeartbeat, startBroadcast, stopBroadcast } from "$lib/api/client";
  import FrequencyDisplay from "$lib/components/FrequencyDisplay.svelte";
  import Waveform from "$lib/components/Waveform.svelte";
  import { joinAsBroadcaster, leaveRoom, setMicEnabled } from "$lib/livekit";
  import {
    broadcastState,
    broadcasterRoom,
    isOnAir,
    ownedChannel,
    resetBroadcastState,
  } from "$lib/stores/channel";
  import { user } from "$lib/stores/user";

  let micMuted = false;
  let statusMessage: string | null = null;
  let heartbeatId: number | null = null;

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
    micMuted = false;
    broadcastState.set("connecting");

    try {
      const session = await startBroadcast($ownedChannel.id);
      const room = await joinAsBroadcaster(session.livekitToken);
      broadcasterRoom.set(room);
      broadcastState.set("live");
      ensureHeartbeatLoop();
    } catch (error) {
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

    micMuted = !micMuted;
    await setMicEnabled($broadcasterRoom, !micMuted);
  }

  $: if ($broadcastState === "live") {
    ensureHeartbeatLoop();
  } else {
    stopHeartbeatLoop();
  }

  onDestroy(() => {
    stopHeartbeatLoop();
  });
</script>

<div class="page-shell">
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

        <div class="controls">
          {#if $isOnAir}
            <button class="ghost-button" type="button" on:click={toggleMic}>
              {micMuted ? "Unmute Mic" : "Mute Mic"}
            </button>
            <button class="danger-button" type="button" on:click={endBroadcast}>
              End Broadcast
            </button>
          {:else}
            <button
              class="primary-button"
              type="button"
              disabled={$broadcastState === "connecting"}
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
          <p>Join the LiveKit room and publish the local microphone.</p>
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

    .broadcast-shell {
      grid-template-columns: minmax(0, 1.25fr) minmax(300px, 0.75fr);
    }
  }
</style>
