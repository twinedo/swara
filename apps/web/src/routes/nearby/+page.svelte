<script lang="ts">
  import { goto } from "$app/navigation";
  import type { Channel, LatLng } from "@swara/types";

  import { fetchNearbyChannels } from "$lib/api/client";
  import ChannelCard from "$lib/components/ChannelCard.svelte";
  import CoverageMap from "$lib/components/CoverageMap.svelte";
  import { formatDistance, formatFrequency } from "$lib/utils/format";
  import { nearbyChannels, selectChannel, setNearbyChannels } from "$lib/stores/channel";
  import {
    activeLocation,
    clearSinggah,
    coverageRadius,
    isPro,
    singgah,
  } from "$lib/stores/location";

  const fallbackChannels: Channel[] = [
    {
      id: "fallback-1",
      frequency: 98.7,
      name: "Kopi Pagi FM",
      status: "live",
      distanceM: 2_100,
      listenerCount: 43,
      owner: { username: "Depok Tengah" },
    },
    {
      id: "fallback-2",
      frequency: 101.2,
      name: "Berita Malam",
      status: "live",
      distanceM: 4_800,
      listenerCount: 12,
      owner: { username: "Margonda" },
    },
    {
      id: "fallback-3",
      frequency: 105.5,
      name: "Indie Sore",
      status: "offline",
      distanceM: 9_300,
      listenerCount: 0,
      owner: { username: "Cinere" },
    },
    {
      id: "fallback-4",
      frequency: 107.0,
      name: "Ngobrol Subuh",
      status: "live",
      distanceM: 11_700,
      listenerCount: 8,
      owner: { username: "Limo" },
    },
    {
      id: "fallback-5",
      frequency: 88.3,
      name: "Senja Radio",
      status: "offline",
      distanceM: 14_100,
      listenerCount: 0,
      owner: { username: "Sawangan" },
    },
  ];

  let liveOnly = false;
  let loading = false;
  let errorMessage: string | null = null;
  let lastLocationKey = "";

  async function loadNearby() {
    if (!$activeLocation) {
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

  function channelLocation(channel: Channel): string {
    return `${channel.owner.username} - ${formatDistance(channel.distanceM)}`;
  }

  $: if ($activeLocation) {
    const nextKey = `${$activeLocation.lat.toFixed(3)}:${$activeLocation.lng.toFixed(3)}:${$coverageRadius}`;
    if (nextKey !== lastLocationKey) {
      lastLocationKey = nextKey;
      void loadNearby();
    }
  }

  $: displayedChannels = liveOnly
    ? $nearbyChannels.filter((channel) => channel.status === "live")
    : $nearbyChannels;
  $: mobileChannels = displayedChannels.length > 0 ? displayedChannels : fallbackChannels;
</script>

<div class="desktop-view">
  <div class="page-shell nearby-shell">
    <section class="map-col">
      <CoverageMap
        center={$activeLocation}
        channels={$nearbyChannels}
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
      {:else}
        <p class="status-copy">Pro unlocks Singgah, so you can scan other areas without moving.</p>
      {/if}

      {#if $isPro}
        <button class="ghost-button clear-button" type="button" on:click={clearSinggah}>
          Clear Singgah
        </button>
      {/if}

      {#if loading}
        <p class="status-copy">Refreshing nearby stations...</p>
      {:else if errorMessage}
        <p class="status-copy">{errorMessage}</p>
      {:else if displayedChannels.length === 0}
        <p class="status-copy">No stations match this view right now.</p>
      {:else}
        <div class="channel-list">
          {#each displayedChannels as channel (channel.id)}
            <ChannelCard channel={channel} on:click={() => handleTune(channel)} />
          {/each}
        </div>
      {/if}
    </section>
  </div>
</div>

<div class="mobile-view">
  <section class="mobile-shell">
    <div class="status-bar">
      <span>09:41</span>
      <span class="status-freq">
        {mobileChannels[0] ? `${formatFrequency(mobileChannels[0].frequency)} FM` : "--.- FM"}
      </span>
      <div class="signal-bars">
        <div class="signal-bar" style="height:3px"></div>
        <div class="signal-bar" style="height:5px"></div>
        <div class="signal-bar" style="height:7px"></div>
      </div>
    </div>

    <div class="nearby-header">
      <div class="nearby-title">NEARBY</div>
      <div class="nearby-radius">within {Math.round($coverageRadius / 1000)} km</div>
    </div>

    {#if loading}
      <p class="mobile-status">Refreshing nearby stations...</p>
    {:else if errorMessage}
      <p class="mobile-status">{errorMessage}</p>
    {:else}
      <div class="nb-list">
        {#each mobileChannels.slice(0, 5) as channel (channel.id)}
          <button
            type="button"
            class:active={$nearbyChannels[0]?.id === channel.id}
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

  .list-col {
    padding: 18px;
    display: grid;
    gap: 16px;
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
    background: var(--bg-primary);
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
    align-items: baseline;
    justify-content: space-between;
    padding: 14px 20px 14px;
  }

  .nearby-title {
    font-family: "Bebas Neue", sans-serif;
    font-size: 30px;
    color: var(--text-primary);
    letter-spacing: 2px;
  }

  .nearby-radius {
    font-family: "Share Tech Mono", monospace;
    font-size: 8px;
    letter-spacing: 1px;
    color: var(--text-muted);
    text-transform: uppercase;
  }

  .mobile-status {
    margin: 0;
    padding: 0 20px;
    color: var(--text-muted);
    font-size: 12px;
  }

  .nb-list {
    flex: 1;
    display: flex;
    flex-direction: column;
  }

  .nb-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 20px;
    border-bottom: 1px solid var(--border-subtle);
    background: transparent;
    color: inherit;
    text-align: left;
    width: 100%;
  }

  .nb-row.active {
    background: #191600;
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
      grid-template-columns: minmax(0, 1.25fr) minmax(320px, 420px);
      padding: 24px;
    }
  }
</style>
