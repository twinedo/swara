<script lang="ts">
  import { goto } from "$app/navigation";
  import type { Channel, LatLng } from "@swara/types";

  import { fetchNearbyChannels } from "$lib/api/client";
  import ChannelCard from "$lib/components/ChannelCard.svelte";
  import CoverageMap from "$lib/components/CoverageMap.svelte";
  import { formatDistance, formatFrequency } from "$lib/utils/format";
  import {
    activeChannel,
    nearbyChannels,
    selectChannel,
    setNearbyChannels,
  } from "$lib/stores/channel";
  import {
    activeLocation,
    clearSinggah,
    coverageRadius,
    isPro,
    singgah,
  } from "$lib/stores/location";
  import { isAuthenticated } from "$lib/stores/user";

  let liveOnly = false;
  let loading = false;
  let errorMessage: string | null = null;
  let lastLocationKey = "";

  async function loadNearby() {
    if (!$isAuthenticated || !$activeLocation) {
      return;
    }

    loading = true;
    errorMessage = null;

    try {
      const channels = await fetchNearbyChannels($activeLocation, $coverageRadius);
      setNearbyChannels(channels);
    } catch (error) {
      errorMessage =
        error instanceof Error ? error.message : "Could not refresh nearby stations.";
    } finally {
      loading = false;
    }
  }

  function handleSinggah(event: CustomEvent<LatLng>) {
    if (singgah(event.detail)) {
      lastLocationKey = "";
    }
  }

  async function handleTune(channel: Channel) {
    selectChannel(channel);
    await goto("/");
  }

  function handleOpenSettings() {
    void goto("/settings");
  }

  function channelLocation(channel: Channel): string {
    return `${channel.owner.username} - ${formatDistance(channel.distanceM)}`;
  }

  $: if (!$isAuthenticated) {
    errorMessage = null;
    loading = false;
    lastLocationKey = "";
    if ($nearbyChannels.length > 0) {
      setNearbyChannels([]);
    }
  }

  $: if ($isAuthenticated && $activeLocation) {
    const nextKey = `${$activeLocation.lat.toFixed(3)}:${$activeLocation.lng.toFixed(3)}:${$coverageRadius}`;
    if (nextKey !== lastLocationKey) {
      lastLocationKey = nextKey;
      void loadNearby();
    }
  }

  $: displayedChannels = liveOnly
    ? $nearbyChannels.filter((channel) => channel.status === "live")
    : $nearbyChannels;
</script>

<div class="desktop-view">
  {#if !$isAuthenticated}
    <div class="page-shell nearby-shell locked-shell">
      <section class="panel access-panel">
        <span class="section-label">Nearby</span>
        <h1>Area Scan Locked</h1>
        <p class="status-copy access-copy">
          Sign in to scan stations around your current area. Pro listeners unlock Singgah, so they
          can hop to another area without moving.
        </p>
        <button class="primary-button" type="button" on:click={handleOpenSettings}>
          Sign In To Scan
        </button>
      </section>
    </div>
  {:else}
    <div class="page-shell nearby-shell">
      <section class="map-col">
        <CoverageMap
          center={$activeLocation}
          channels={displayedChannels}
          radiusMetres={$coverageRadius}
          allowSinggah={$isPro}
          on:singgah={handleSinggah}
        />
      </section>

      <section class="list-col panel">
        <div class="head">
          <div>
            <span class="section-label">Nearby</span>
            <h1>Area Scan</h1>
          </div>

          <label class="toggle mono">
            <input bind:checked={liveOnly} type="checkbox" />
            Live Only
          </label>
        </div>

        {#if $isPro}
          <div class="notice">
            <div>⌖</div>
            <div>
              <strong>Singgah</strong>
              <p>Tap anywhere on the map to hop your listening radius to another part of town.</p>
            </div>
          </div>

          <button class="ghost-button clear-button" type="button" on:click={clearSinggah}>
            Clear Singgah
          </button>
        {:else}
          <p class="status-copy">
            Your scan stays locked to your current area. Pro unlocks Singgah for remote scanning.
          </p>
        {/if}

        {#if !$activeLocation}
          <p class="status-copy">Enable location access to scan nearby radio in your area.</p>
        {:else if loading}
          <p class="status-copy">Refreshing nearby stations...</p>
        {:else if errorMessage}
          <p class="status-copy">{errorMessage}</p>
        {:else if displayedChannels.length === 0}
          <p class="status-copy">No stations match this view right now.</p>
        {:else}
          <div class="channel-list">
            {#each displayedChannels as channel (channel.id)}
              <ChannelCard
                channel={channel}
                active={$activeChannel?.id === channel.id}
                on:click={() => handleTune(channel)}
              />
            {/each}
          </div>
        {/if}
      </section>
    </div>
  {/if}
</div>

<div class="mobile-view">
  <section class="mobile-shell">
    <div class="status-bar">
      <span>09:41</span>
      <span class="status-freq">
        {$activeChannel ? `${formatFrequency($activeChannel.frequency)} FM` : "AREA SCAN"}
      </span>
      <div class="signal-bars">
        <div class="signal-bar" style="height:3px"></div>
        <div class="signal-bar" style="height:5px"></div>
        <div class="signal-bar" style="height:7px"></div>
      </div>
    </div>

    <div class="nearby-header">
      <div>
        <div class="nearby-title">NEARBY</div>
        <div class="nearby-subtitle">AREA SCAN</div>
      </div>
      <div class="nearby-radius">within {Math.round($coverageRadius / 1000)} km</div>
    </div>

    {#if !$isAuthenticated}
      <div class="mobile-panel mobile-access">
        <div class="mobile-copy">
          <strong>Nearby needs an account</strong>
          <p>Sign in to scan radio around your current area. Pro unlocks Singgah for remote scans.</p>
        </div>
        <button class="primary-button mobile-cta" type="button" on:click={handleOpenSettings}>
          Sign In
        </button>
      </div>
    {:else}
      <div class="mobile-map">
        <CoverageMap
          center={$activeLocation}
          channels={displayedChannels}
          radiusMetres={$coverageRadius}
          allowSinggah={$isPro}
          compact={true}
          on:singgah={handleSinggah}
        />
      </div>

      <div class="mobile-panel mobile-controls">
        <label class="toggle mono">
          <input bind:checked={liveOnly} type="checkbox" />
          Live Only
        </label>

        {#if $isPro}
          <button class="ghost-button mini-action" type="button" on:click={clearSinggah}>
            Clear Singgah
          </button>
        {/if}
      </div>

      {#if $isPro}
        <p class="mobile-status">
          Pro is active. Tap the coverage map to hop your scan to another nearby area.
        </p>
      {:else}
        <p class="mobile-status">
          Scan is locked to your current area. Pro unlocks Singgah for remote scanning.
        </p>
      {/if}

      {#if !$activeLocation}
        <p class="mobile-status">Enable location access to scan nearby radio in your area.</p>
      {:else if loading}
        <p class="mobile-status">Refreshing nearby stations...</p>
      {:else if errorMessage}
        <p class="mobile-status">{errorMessage}</p>
      {:else if displayedChannels.length === 0}
        <p class="mobile-status">No stations match this view right now.</p>
      {:else}
        <div class="nb-list">
          {#each displayedChannels as channel (channel.id)}
            <button
              type="button"
              class:active={$activeChannel?.id === channel.id}
              class="nb-row"
              on:click={() => handleTune(channel)}
            >
              <div class:dim={channel.status !== "live"} class="nb-freq">
                {formatFrequency(channel.frequency)}
              </div>
              <div class="nb-info">
                <div class:dim={channel.status !== "live"} class="nb-name">{channel.name}</div>
                <div class:dim={channel.status !== "live"} class="nb-loc">{channelLocation(channel)}</div>
              </div>
              <div class:live={channel.status === "live"} class:off={channel.status !== "live"} class="pill">
                {channel.status === "live" ? "LIVE" : "OFF"}
              </div>
            </button>
          {/each}
        </div>
      {/if}
    {/if}
  </section>
</div>

<style>
  .desktop-view {
    display: none;
  }

  .nearby-shell {
    display: grid;
    gap: 18px;
    padding: 18px 16px 0;
  }

  .locked-shell {
    grid-template-columns: minmax(0, 1fr);
    min-height: calc(100vh - 72px - var(--sab));
    place-items: center;
  }

  .access-panel,
  .list-col {
    padding: 18px;
    display: grid;
    gap: 16px;
  }

  .access-panel {
    max-width: 680px;
    width: min(100%, 680px);
  }

  .access-copy {
    max-width: 56ch;
  }

  .head {
    display: flex;
    justify-content: space-between;
    gap: 16px;
    align-items: flex-start;
  }

  h1 {
    margin: 8px 0 0;
    font-family: "Bebas Neue", sans-serif;
    font-size: 38px;
    letter-spacing: 2px;
  }

  .toggle {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--text-muted);
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 1.4px;
  }

  .channel-list {
    display: grid;
    gap: 10px;
  }

  .clear-button {
    width: fit-content;
  }

  .mobile-view {
    display: block;
  }

  .mobile-shell {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    gap: 14px;
    background: var(--bg-primary);
    padding-bottom: calc(84px + var(--sab));
  }

  .status-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: calc(8px + var(--sat)) 16px 4px;
    font-family: "Share Tech Mono", monospace;
    font-size: 8px;
    color: var(--text-muted);
    letter-spacing: 1px;
  }

  .status-freq {
    color: var(--accent);
    font-size: 8px;
    letter-spacing: 1px;
  }

  .signal-bars {
    display: flex;
    gap: 1.5px;
    align-items: flex-end;
    height: 8px;
  }

  .signal-bar {
    width: 3px;
    background: var(--text-muted);
    border-radius: 1px;
  }

  .nearby-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    padding: 14px 20px 0;
  }

  .nearby-title {
    font-family: "Bebas Neue", sans-serif;
    font-size: 30px;
    color: var(--text-primary);
    letter-spacing: 2px;
  }

  .nearby-subtitle {
    margin-top: 2px;
    font-family: "Share Tech Mono", monospace;
    font-size: 8px;
    letter-spacing: 1.8px;
    color: var(--text-muted);
    text-transform: uppercase;
  }

  .nearby-radius {
    font-family: "Share Tech Mono", monospace;
    font-size: 8px;
    letter-spacing: 1px;
    color: var(--text-muted);
    text-transform: uppercase;
    padding-top: 12px;
  }

  .mobile-map,
  .mobile-panel,
  .mobile-status {
    margin: 0 14px;
  }

  .mobile-panel {
    padding: 14px;
    border: 1px solid var(--border-default);
    border-radius: 16px;
    background: linear-gradient(180deg, rgba(28, 28, 28, 0.94), rgba(18, 18, 18, 0.94));
  }

  .mobile-access,
  .mobile-controls {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .mobile-copy strong {
    display: block;
    margin-bottom: 4px;
    font-family: "Share Tech Mono", monospace;
    font-size: 10px;
    letter-spacing: 1.4px;
    color: var(--accent);
    text-transform: uppercase;
  }

  .mobile-copy p {
    margin: 0;
    color: var(--text-muted);
    font-size: 12px;
    line-height: 1.45;
  }

  .mobile-cta,
  .mini-action {
    flex-shrink: 0;
  }

  .mobile-status {
    color: var(--text-muted);
    font-size: 12px;
    line-height: 1.5;
  }

  .nb-list {
    flex: 1;
    display: flex;
    flex-direction: column;
    margin-top: 2px;
  }

  .nb-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 20px;
    border-top: 1px solid rgba(255, 255, 255, 0.08);
    background: transparent;
    color: inherit;
    text-align: left;
    width: 100%;
  }

  .nb-row:last-child {
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }

  .nb-row.active {
    background: rgba(232, 200, 74, 0.06);
  }

  .nb-freq {
    font-family: "Bebas Neue", sans-serif;
    font-size: 20px;
    color: var(--accent);
    line-height: 1;
    min-width: 42px;
  }

  .nb-freq.dim {
    color: var(--text-muted);
  }

  .nb-info {
    flex: 1;
  }

  .nb-name {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .nb-name.dim {
    color: var(--text-muted);
  }

  .nb-loc {
    font-family: "Share Tech Mono", monospace;
    font-size: 7px;
    letter-spacing: 1px;
    color: var(--text-muted);
    margin-top: 1px;
    text-transform: uppercase;
  }

  .nb-loc.dim {
    color: var(--text-disabled);
  }

  .pill {
    font-family: "Share Tech Mono", monospace;
    font-size: 7px;
    letter-spacing: 1px;
    padding: 3px 8px;
    border-radius: 12px;
  }

  .pill.live {
    background: #0a2a0a;
    color: var(--online);
    border: 1px solid #1a4a1a;
  }

  .pill.off {
    background: var(--bg-card);
    color: var(--text-disabled);
    border: 1px solid var(--border-default);
  }

  @media (min-width: 1024px) {
    .desktop-view {
      display: block;
    }

    .mobile-view {
      display: none;
    }

    .nearby-shell {
      grid-template-columns: minmax(280px, 1fr) minmax(0, 2fr);
      padding: 24px;
    }

    .locked-shell {
      grid-template-columns: minmax(0, 1fr);
      justify-items: center;
    }
  }
</style>
