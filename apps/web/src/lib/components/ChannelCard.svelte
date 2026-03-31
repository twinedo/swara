<script lang="ts">
  import type { Channel } from "@swara/types";

  import Waveform from "$lib/components/Waveform.svelte";
  import {
    buildChannelSubline,
    formatDistance,
    formatFrequency,
    formatListeners,
  } from "$lib/utils/format";

  export let channel: Channel;
  export let active = false;
  export let currentShow: string | null = null;
</script>

<button class:active class:offline={channel.status !== "live"} class="channel-card">
  <div class="freq mono">{formatFrequency(channel.frequency)}</div>

  <div class="body">
    <div class="title-row">
      <strong>{channel.name}</strong>
      {#if channel.status === "live"}
        <span class="live-pill">Live</span>
      {:else}
        <span class="offline-pill">Offline</span>
      {/if}
    </div>

    <p class="subline">{buildChannelSubline(channel)}</p>

    {#if currentShow}
      <p class="showline">{currentShow}</p>
    {/if}
  </div>

  <div class="tail">
    {#if channel.status === "live"}
      <Waveform active={active} compact={true} />
      <span class="listeners mono">{formatListeners(channel.listenerCount)} listening</span>
    {:else}
      <span class="listeners mono">{formatDistance(channel.distanceM)}</span>
    {/if}
  </div>
</button>

<style>
  .channel-card {
    width: 100%;
    display: grid;
    grid-template-columns: auto 1fr auto;
    gap: 14px;
    align-items: center;
    padding: 14px 16px;
    border-radius: 18px;
    border: 1px solid var(--border-default);
    background: rgba(255, 255, 255, 0.02);
    color: inherit;
    text-align: left;
    transition:
      background 180ms ease,
      border-color 180ms ease,
      transform 180ms ease;
  }

  .channel-card:hover {
    transform: translateY(-1px);
    border-color: rgba(232, 200, 74, 0.24);
    background: rgba(232, 200, 74, 0.04);
  }

  .channel-card.active {
    border-color: rgba(232, 200, 74, 0.34);
    background: linear-gradient(180deg, rgba(232, 200, 74, 0.1), rgba(255, 255, 255, 0.03));
  }

  .channel-card.offline {
    opacity: 0.8;
  }

  .freq {
    font-family: "Bebas Neue", sans-serif;
    font-size: 34px;
    line-height: 1;
    letter-spacing: 1px;
    color: var(--accent);
  }

  .body {
    min-width: 0;
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }

  strong {
    font-size: 15px;
  }

  .subline,
  .showline {
    margin: 4px 0 0;
    font-size: 12px;
    color: var(--text-muted);
  }

  .showline {
    color: var(--text-primary);
  }

  .tail {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 6px;
    justify-self: end;
  }

  .listeners {
    font-size: 10px;
    letter-spacing: 1px;
    color: var(--text-muted);
    text-transform: uppercase;
    white-space: nowrap;
  }

  .offline-pill {
    display: inline-flex;
    padding: 4px 8px;
    border-radius: 999px;
    border: 1px solid var(--border-default);
    color: var(--text-disabled);
    font-family: "Share Tech Mono", monospace;
    font-size: 9px;
    letter-spacing: 1.2px;
    text-transform: uppercase;
  }
</style>
