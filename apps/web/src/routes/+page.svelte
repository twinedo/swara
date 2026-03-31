<script lang="ts">
  import type { Channel } from "@swara/types";
  import { get } from "svelte/store";

  import {
    fetchNearbyChannels,
    fetchSchedule,
    startListening,
  } from "$lib/api/client";
  import { joinAsListener, leaveRoom } from "$lib/livekit";
  import { clearMediaSession, setMediaSession, setPlaybackState } from "$lib/mediaSession";
  import {
    activeChannel,
    activeSchedule,
    listenerRoom,
    nearbyChannels,
    playbackState,
    resetListeningState,
    selectChannel,
    setNearbyChannels,
  } from "$lib/stores/channel";
  import { activeLocation, coverageRadius, locationError } from "$lib/stores/location";
  import { isAuthenticated } from "$lib/stores/user";
  import { formatDistance, formatFrequency, getCurrentShow } from "$lib/utils/format";

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

  const fallbackSchedule = [
    { startTime: "06:00", showName: "Morning Talk", hostName: "Raka & Dina" },
    { startTime: "09:00", showName: "Kopi Sambil Kerja", hostName: "Solo show - Bimo" },
    { startTime: "12:00", showName: "Siang Santai", hostName: "Community call-in" },
  ];

  const desktopWave = [7, 13, 5, 17, 9, 15, 19, 11, 7, 15, 17, 5, 13, 9, 7, 17, 11, 15];
  const mobileWave = [5, 11, 7, 15, 9, 13, 5, 11, 15, 7];
  const radarPositions = [
    { top: "28%", left: "54%" },
    { top: "16%", left: "63%" },
    { top: "56%", left: "63%" },
    { top: "73%", left: "44%" },
  ];

  let isLoading = false;
  let errorMessage: string | null = null;
  let lastLocationKey = "";
  let lastScheduleChannelId = "";

  async function loadNearby() {
    if (!$activeLocation) {
      return;
    }

    isLoading = true;
    errorMessage = null;

    try {
      const channels = await fetchNearbyChannels($activeLocation, $coverageRadius);
      setNearbyChannels(channels);
    } catch (error) {
      errorMessage =
        error instanceof Error ? error.message : "Could not load nearby channels.";
    } finally {
      isLoading = false;
    }
  }

  async function loadSchedule(channelId: string) {
    try {
      const schedule = await fetchSchedule(channelId);
      activeSchedule.set(schedule);
    } catch {
      activeSchedule.set([]);
    }
  }

  async function stopListening() {
    await leaveRoom(get(listenerRoom));
    resetListeningState();
    selectChannel(null);
    clearMediaSession();
    setPlaybackState("none");
  }

  async function tune(channel: Channel) {
    errorMessage = null;

    if (get(activeChannel)?.id === channel.id && get(playbackState) === "playing") {
      await stopListening();
      return;
    }

    selectChannel(channel);

    if (channel.status !== "live") {
      playbackState.set("idle");
      return;
    }

    if (!get(isAuthenticated)) {
      playbackState.set("error");
      errorMessage = "Sign in first. The current API requires an authenticated listener token.";
      return;
    }

    await leaveRoom(get(listenerRoom));
    listenerRoom.set(null);
    playbackState.set("connecting");

    try {
      const session = await startListening(channel.id);
      const room = await joinAsListener(session.livekitToken, session.roomName);
      listenerRoom.set(room);
      selectChannel({
        ...channel,
        listenerCount: session.channel.listenerCount,
      });
      playbackState.set("playing");
      setMediaSession(
        {
          title: channel.name,
          subtitle: `${channel.frequency.toFixed(1)} FM`,
          artworkSrc: "/icons/icon.svg",
        },
        {
          onPlay: () => setPlaybackState("playing"),
          onPause: () => setPlaybackState("paused"),
          onStop: () => {
            void stopListening();
          },
        },
      );
      setPlaybackState("playing");
    } catch (error) {
      playbackState.set("error");
      errorMessage = error instanceof Error ? error.message : "Could not start live audio.";
    }
  }

  function channelDistance(channel: Channel): string {
    return formatDistance(channel.distanceM);
  }

  function channelLocation(channel: Channel): string {
    return `${channel.owner.username} - ${channelDistance(channel)}`;
  }

  function waveStyle(height: number, index: number): string {
    const delay = (index * 0.04).toFixed(2);
    const duration = (0.36 + (index % 6) * 0.03).toFixed(2);
    return `height:${height}px;animation-delay:${delay}s;animation-duration:${duration}s`;
  }

  function pinStyle(index: number): string {
    const point = radarPositions[index] ?? radarPositions[0];
    return `top:${point.top};left:${point.left};`;
  }

  function primaryActionLabel(state: string, channel: Channel | null): string {
    if (!channel) {
      return "TUNE IN";
    }

    if (state === "playing" && get(activeChannel)?.id === channel.id) {
      return "STOP";
    }

    if (state === "connecting") {
      return "CONNECTING";
    }

    return "TUNE IN";
  }

  function tunePrimary(): void {
    if (displayChannel) {
      void tune(displayChannel);
    }
  }

  $: if ($activeLocation) {
    const nextKey = `${$activeLocation.lat.toFixed(3)}:${$activeLocation.lng.toFixed(3)}:${$coverageRadius}`;
    if (nextKey !== lastLocationKey) {
      lastLocationKey = nextKey;
      void loadNearby();
    }
  }

  $: if ($activeChannel?.id) {
    if ($activeChannel.id !== lastScheduleChannelId) {
      lastScheduleChannelId = $activeChannel.id;
      void loadSchedule($activeChannel.id);
    }
  } else {
    activeSchedule.set([]);
  }

  $: currentShow = getCurrentShow($activeSchedule);
  $: displayChannels = $nearbyChannels.length > 0 ? $nearbyChannels : fallbackChannels;
  $: displayChannel = $activeChannel ?? displayChannels[0] ?? null;
  $: mapChannels = displayChannels.slice(0, 4);
  $: scheduleRows = $activeSchedule.length > 0 ? $activeSchedule : fallbackSchedule;
  $: primaryLabel = primaryActionLabel($playbackState, displayChannel);
</script>

<div class="desktop-view">
  <section class="desktop-wrap">
    <div class="desktop-chrome">
      <div class="chrome-dot r"></div>
      <div class="chrome-dot y"></div>
      <div class="chrome-dot g"></div>
      <div class="chrome-url"><span>swara.fm</span></div>
    </div>

    <div class="app-shell">
      <div class="app-topbar">
        <div class="app-brand">SWARA <span>// REGIONAL RADIO</span></div>
        <div class="on-air-badge"><div class="on-air-dot"></div> ON AIR</div>
      </div>

      <div class="left-col">
        <div class="tuned-card">
          <div class="card-label">NOW TUNED</div>
          <div class="tuned-freq-row">
            <div class="tuned-freq">{displayChannel ? formatFrequency(displayChannel.frequency) : "--.-"}</div>
            <div class="tuned-fm">FM</div>
          </div>
          <div class="tuned-meta">
            <span>REGION <b>{displayChannel ? displayChannel.owner.username : "Depok, ID"}</b></span>
            <span>RADIUS <b>{Math.round($coverageRadius / 1000)} km</b></span>
            <span>LISTENERS <b>{displayChannel ? displayChannel.listenerCount : 0}</b></span>
          </div>
          <div class="waveform" aria-hidden="true">
            {#each desktopWave as bar, index}
              <div class="wv" style={waveStyle(bar, index)}></div>
            {/each}
          </div>
        </div>

        <div class="channels-label">NEARBY CHANNELS</div>

        {#if isLoading}
          <p class="status-copy">Scanning your area for nearby stations...</p>
        {:else}
          <div class="ch-list">
            {#each displayChannels.slice(0, 5) as channel (channel.id)}
              <button
                type="button"
                class:active={$activeChannel?.id === channel.id}
                class:offline={channel.status !== "live"}
                class="ch-row"
                on:click={() => tune(channel)}
              >
                <div class:dim={channel.status !== "live"} class="ch-freq">
                  {formatFrequency(channel.frequency)}
                </div>
                <div class="ch-info">
                  <div class:dim={channel.status !== "live"} class="ch-name">
                    {#if channel.status === "live"}
                      <span class="live-dot-sm"></span>
                    {/if}
                    {channel.name}
                  </div>
                  <div class:dim={channel.status !== "live"} class="ch-sub">{channelLocation(channel)}</div>
                </div>
                {#if channel.status === "live"}
                  <div class="ch-stat">
                    <span class="count">{channel.listenerCount}</span>
                    <span class="word">listening</span>
                  </div>
                {:else}
                  <div class="ch-stat offline">- offline</div>
                {/if}
              </button>
            {/each}
          </div>
        {/if}

        {#if $locationError}
          <p class="status-copy">Location: {$locationError}</p>
        {/if}
      </div>

      <div class="right-col">
        <div class="map-card">
          <div class="card-label">COVERAGE MAP</div>
          <div class="map-radar">
            <div class="map-ring r1"></div>
            <div class="map-ring r2"></div>
            <div class="map-ring r3"></div>
            <div class="map-you-dot"></div>
            <div class="map-you-lbl">YOU</div>

            {#each mapChannels as channel, index (channel.id)}
              <div class="map-pin" style={pinStyle(index)}>
                <div class:live={channel.status === "live"} class:off={channel.status !== "live"} class="map-pin-dot"></div>
                <div class:off-label={channel.status !== "live"} class="map-pin-lbl">
                  {formatFrequency(channel.frequency)}
                </div>
              </div>
            {/each}
          </div>
          <div class="map-radius-lbl">{Math.round($coverageRadius / 1000)} km radius</div>
        </div>

        <div class="np-card">
          <div class="np-tag">NOW PLAYING</div>
          <div class="np-title">{currentShow?.showName ?? "Morning Talk - Eps 34"}</div>
          <div class="np-by">by {currentShow?.hostName ?? "Raka & Dina"}</div>
          <div class="np-btns">
            <button class="btn-tune" type="button" disabled={!displayChannel} on:click={tunePrimary}>
              {primaryLabel}
            </button>
            <button class="btn-share" type="button">SHARE</button>
          </div>
          <div class="vol-row">
            <div class="vol-lbl">VOL</div>
            <div class="vol-track">
              <div class="vol-fill"></div>
              <div class="vol-thumb"></div>
            </div>
          </div>
        </div>

        <div>
          <div class="sched-label">TODAY'S SCHEDULE</div>
          {#each scheduleRows.slice(0, 4) as entry}
            <div class="sched-row">
              <div class="sched-time">{entry.startTime}</div>
              <div>
                <div class="sched-show">{entry.showName}</div>
                <div class="sched-host">{entry.hostName ?? "Community hosted"}</div>
              </div>
            </div>
          {/each}
        </div>
      </div>
    </div>
  </section>
</div>

<div class="mobile-view">
  <section class="mobile-shell">
    <div class="status-bar">
      <span>09:41</span>
      <span class="status-freq">{displayChannel ? formatFrequency(displayChannel.frequency) : "--.-"} FM</span>
      <div class="signal-bars">
        <div class="signal-bar" style="height:3px"></div>
        <div class="signal-bar" style="height:5px"></div>
        <div class="signal-bar" style="height:7px"></div>
      </div>
    </div>

    <div class="tune-screen">
      <div class="tune-stack">
        <div class="tune-np-label">
          NOW PLAYING
          <div class="live-badge"><div class="live-badge-dot"></div>LIVE</div>
        </div>

        <div class="tune-hero">
          <div class="tune-freq-big">{displayChannel ? formatFrequency(displayChannel.frequency) : "--.-"}</div>
          <div class="tune-fm">FM</div>
        </div>
        <div class="tune-name">{displayChannel?.name ?? "Kopi Pagi FM"}</div>
        <div class="tune-sub">
          {displayChannel ? `${channelLocation(displayChannel)} away` : "Depok Tengah - 2.1 km away"}
        </div>

        <div class="tune-wv" aria-hidden="true">
          {#each mobileWave as bar, index}
            <div class="wv" style={waveStyle(bar, index)}></div>
          {/each}
        </div>

        <div class="tune-controls">
          <div class="ctrl-skip">-15</div>
          <button
            type="button"
            class="ctrl-play"
            aria-label="Tune in"
            disabled={!displayChannel}
            on:click={tunePrimary}
          >
            <div class="play-tri"></div>
          </button>
          <div class="ctrl-heart">FAV</div>
        </div>
        <div class="tune-listeners">{(displayChannel?.listenerCount ?? 0).toString()} people tuned in nearby</div>

        <div class="lock-notice">
          <div class="lock-notice-icon">LOCK</div>
          <div>
            <div class="lock-notice-title">{errorMessage ? "Playback notice" : "Lock screen controls active"}</div>
            <div class="lock-notice-sub">{errorMessage ?? "Audio keeps playing in background"}</div>
          </div>
        </div>
      </div>
    </div>
  </section>
</div>

<style>
  .desktop-view {
    display: none;
  }

  .desktop-wrap {
    margin: 24px;
    background: #000;
    border-radius: 14px;
    border: 2px solid #2a2a2a;
    overflow: hidden;
  }

  .desktop-chrome {
    background: #0a0a0a;
    padding: 9px 14px;
    display: flex;
    align-items: center;
    gap: 7px;
    border-bottom: 1px solid #1a1a1a;
  }

  .chrome-dot {
    width: 11px;
    height: 11px;
    border-radius: 50%;
  }

  .chrome-dot.r {
    background: #e84a4a;
  }

  .chrome-dot.y {
    background: #e8c84a;
  }

  .chrome-dot.g {
    background: #4ae87a;
  }

  .chrome-url {
    flex: 1;
    background: #161616;
    border-radius: 5px;
    height: 22px;
    display: flex;
    align-items: center;
    padding: 0 10px;
    margin-left: 8px;
  }

  .chrome-url span {
    font-size: 10px;
    color: var(--text-muted);
  }

  .app-shell {
    background: var(--bg-primary);
    display: grid;
    grid-template-columns: 1fr 1fr;
    min-height: 580px;
  }

  .app-topbar {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 28px;
    border-bottom: 1px solid var(--border-default);
  }

  .app-brand {
    font-family: "Bebas Neue", sans-serif;
    font-size: 20px;
    color: var(--accent);
    letter-spacing: 2px;
  }

  .app-brand span {
    color: var(--text-muted);
    font-size: 14px;
    letter-spacing: 3px;
    margin-left: 4px;
  }

  .on-air-badge {
    display: flex;
    align-items: center;
    gap: 7px;
    font-family: "Share Tech Mono", monospace;
    font-size: 10px;
    letter-spacing: 2px;
    color: var(--live);
  }

  .on-air-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--live);
    animation: blink 1.2s infinite;
  }

  .left-col {
    padding: 24px 22px 24px 28px;
    border-right: 1px solid var(--border-default);
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .right-col {
    padding: 24px 28px 24px 22px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .tuned-card {
    background: var(--bg-card);
    border: 1px solid var(--border-default);
    border-radius: 10px;
    padding: 20px 22px;
  }

  .card-label {
    font-family: "Share Tech Mono", monospace;
    font-size: 9px;
    letter-spacing: 2px;
    color: var(--text-muted);
    margin-bottom: 6px;
  }

  .tuned-freq-row {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin-bottom: 8px;
  }

  .tuned-freq {
    font-family: "Bebas Neue", sans-serif;
    font-size: 64px;
    color: var(--accent);
    line-height: 1;
  }

  .tuned-fm {
    font-family: "Share Tech Mono", sans-serif;
    font-size: 14px;
    color: var(--text-muted);
    letter-spacing: 2px;
  }

  .tuned-meta {
    display: flex;
    gap: 16px;
    margin-bottom: 14px;
    font-family: "Share Tech Mono", monospace;
    font-size: 9px;
    letter-spacing: 1px;
    color: var(--text-muted);
  }

  .tuned-meta b {
    color: var(--text-primary);
    font-weight: 400;
  }

  .waveform {
    display: flex;
    align-items: center;
    gap: 2.5px;
    height: 20px;
  }

  .wv {
    width: 3px;
    background: var(--accent);
    border-radius: 2px;
    animation: wave ease-in-out infinite alternate;
  }

  .channels-label {
    font-family: "Share Tech Mono", monospace;
    font-size: 9px;
    letter-spacing: 2px;
    color: var(--text-muted);
    text-transform: uppercase;
  }

  .ch-list {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .ch-row {
    background: var(--bg-card);
    border: 1px solid var(--border-default);
    border-radius: 8px;
    padding: 11px 16px;
    display: flex;
    align-items: center;
    gap: 14px;
    text-align: left;
    color: inherit;
    width: 100%;
  }

  .ch-row.active {
    border-color: #e8c84a33;
    background: #191600;
  }

  .ch-row.offline {
    opacity: 0.85;
  }

  .ch-freq {
    font-family: "Bebas Neue", sans-serif;
    font-size: 32px;
    color: var(--accent);
    line-height: 1;
    min-width: 52px;
  }

  .ch-freq.dim {
    color: var(--text-muted);
  }

  .ch-info {
    flex: 1;
    min-width: 0;
  }

  .ch-name {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .ch-name.dim {
    color: var(--text-muted);
  }

  .ch-sub {
    font-family: "Share Tech Mono", monospace;
    font-size: 8px;
    letter-spacing: 1px;
    color: var(--text-muted);
    margin-top: 2px;
  }

  .ch-sub.dim {
    color: var(--text-disabled);
  }

  .live-dot-sm {
    display: inline-block;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--online);
    margin-right: 5px;
    animation: blink 1.5s infinite;
    vertical-align: middle;
  }

  .ch-stat {
    font-family: "Share Tech Mono", monospace;
    font-size: 9px;
    letter-spacing: 1px;
    text-align: right;
  }

  .ch-stat .count {
    color: var(--text-primary);
  }

  .ch-stat .word {
    color: var(--text-muted);
    display: block;
  }

  .ch-stat.offline {
    color: var(--text-disabled);
  }

  .map-card {
    background: var(--bg-card);
    border: 1px solid var(--border-default);
    border-radius: 10px;
    padding: 16px 20px;
  }

  .map-radar {
    width: 160px;
    height: 160px;
    border-radius: 50%;
    background: radial-gradient(circle, #0d1a0d 0%, #050f05 100%);
    border: 1px solid #1a2a1a;
    position: relative;
    margin: 12px auto 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .map-ring {
    position: absolute;
    border-radius: 50%;
    border: 1px solid #1e2e1e;
  }

  .map-ring.r1 {
    width: 55%;
    height: 55%;
  }

  .map-ring.r2 {
    width: 80%;
    height: 80%;
  }

  .map-ring.r3 {
    width: 105%;
    height: 105%;
    opacity: 0.4;
  }

  .map-you-dot {
    position: absolute;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--accent);
    border: 2px solid #050f05;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    z-index: 2;
  }

  .map-you-lbl {
    position: absolute;
    font-family: "Share Tech Mono", monospace;
    font-size: 7px;
    color: var(--accent);
    letter-spacing: 1px;
    top: calc(50% + 8px);
    left: calc(50% + 3px);
  }

  .map-pin {
    position: absolute;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
  }

  .map-pin-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    border: 1px solid #050f05;
  }

  .map-pin-dot.live {
    background: var(--online);
    animation: blink 1.5s infinite;
  }

  .map-pin-dot.off {
    background: var(--text-disabled);
  }

  .map-pin-lbl {
    font-family: "Share Tech Mono", monospace;
    font-size: 7px;
    color: var(--text-primary);
    background: #0d1a0dcc;
    padding: 1px 3px;
    border-radius: 2px;
    white-space: nowrap;
  }

  .map-pin-lbl.off-label {
    color: var(--text-disabled);
  }

  .map-radius-lbl {
    font-family: "Share Tech Mono", monospace;
    font-size: 8px;
    color: var(--text-muted);
    letter-spacing: 1px;
    text-align: center;
    margin-top: 10px;
  }

  .np-card {
    background: var(--bg-card);
    border: 1px solid var(--border-default);
    border-radius: 10px;
    padding: 18px 20px;
  }

  .np-tag {
    font-family: "Share Tech Mono", monospace;
    font-size: 9px;
    letter-spacing: 2px;
    color: var(--live);
    margin-bottom: 8px;
  }

  .np-title {
    font-size: 30px;
    font-weight: 500;
    color: var(--text-primary);
    margin-bottom: 2px;
    line-height: 1.1;
  }

  .np-by {
    font-size: 12px;
    color: var(--text-muted);
    margin-bottom: 14px;
  }

  .np-btns {
    display: flex;
    gap: 10px;
    margin-bottom: 14px;
  }

  .btn-tune {
    font-family: "Bebas Neue", sans-serif;
    font-size: 14px;
    letter-spacing: 2px;
    color: var(--bg-primary);
    background: var(--accent);
    border: none;
    padding: 8px 20px;
    border-radius: 5px;
    min-width: 88px;
  }

  .btn-tune:disabled {
    opacity: 0.6;
  }

  .btn-share {
    font-family: "Bebas Neue", sans-serif;
    font-size: 14px;
    letter-spacing: 2px;
    color: var(--text-primary);
    background: transparent;
    border: 1px solid var(--border-default);
    padding: 8px 20px;
    border-radius: 5px;
  }

  .vol-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .vol-lbl {
    font-family: "Share Tech Mono", monospace;
    font-size: 8px;
    color: var(--text-muted);
    letter-spacing: 1px;
  }

  .vol-track {
    flex: 1;
    height: 3px;
    background: var(--border-default);
    border-radius: 3px;
    position: relative;
  }

  .vol-fill {
    width: 70%;
    height: 100%;
    background: var(--text-muted);
    border-radius: 3px;
  }

  .vol-thumb {
    position: absolute;
    right: 30%;
    top: 50%;
    transform: translate(50%, -50%);
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--text-primary);
    border: 2px solid var(--bg-primary);
  }

  .sched-label {
    font-family: "Share Tech Mono", monospace;
    font-size: 9px;
    letter-spacing: 2px;
    color: var(--text-muted);
    margin-bottom: 12px;
  }

  .sched-row {
    display: flex;
    gap: 12px;
    padding: 6px 0;
    border-bottom: 1px solid var(--border-subtle);
  }

  .sched-row:last-child {
    border-bottom: none;
  }

  .sched-time {
    font-family: "Share Tech Mono", monospace;
    font-size: 9px;
    color: var(--text-muted);
    letter-spacing: 1px;
    min-width: 36px;
    padding-top: 1px;
  }

  .sched-show {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .sched-host {
    font-family: "Share Tech Mono", monospace;
    font-size: 8px;
    color: var(--text-muted);
    letter-spacing: 1px;
    margin-top: 2px;
  }

  .status-copy {
    margin: 0;
    color: var(--text-muted);
    font-size: 12px;
  }

  .mobile-view {
    min-height: 100vh;
    display: block;
  }

  .mobile-shell {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--bg-primary);
  }

  .tune-screen {
    flex: 1;
    min-height: 0;
  }

  .tune-screen {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 12px 0 20px;
  }

  .tune-stack {
    width: 100%;
    max-width: 360px;
    margin: 0 auto;
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

  .tune-np-label {
    font-family: "Share Tech Mono", monospace;
    font-size: 9px;
    letter-spacing: 2px;
    color: var(--text-muted);
    padding: 12px 20px 4px;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .live-badge {
    display: flex;
    align-items: center;
    gap: 5px;
    background: var(--live);
    border-radius: 3px;
    padding: 3px 8px;
    font-family: "Share Tech Mono", monospace;
    font-size: 7px;
    letter-spacing: 2px;
    color: #fff;
  }

  .live-badge-dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: #fff;
    animation: blink 1.2s infinite;
  }

  .tune-hero {
    text-align: center;
    padding: 6px 20px 0;
  }

  .tune-freq-big {
    font-family: "Bebas Neue", sans-serif;
    font-size: clamp(72px, 24vw, 86px);
    color: var(--accent);
    line-height: 1;
  }

  .tune-fm {
    font-family: "Share Tech Mono", monospace;
    font-size: 16px;
    color: var(--text-muted);
    letter-spacing: 3px;
    text-align: center;
  }

  .tune-name {
    font-size: 16px;
    font-weight: 500;
    color: var(--text-primary);
    text-align: center;
    margin: 4px 0 1px;
  }

  .tune-sub {
    font-family: "Share Tech Mono", monospace;
    font-size: 7px;
    letter-spacing: 1px;
    color: var(--text-muted);
    text-align: center;
    margin-bottom: 10px;
    text-transform: uppercase;
  }

  .tune-wv {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 2px;
    height: 16px;
    margin-bottom: 18px;
  }

  .tune-controls {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 22px;
    margin-bottom: 6px;
  }

  .ctrl-skip,
  .ctrl-heart {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    background: var(--bg-card);
    border: 1px solid var(--border-default);
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: "Share Tech Mono", monospace;
    font-size: 8px;
    color: var(--text-muted);
  }

  .ctrl-play {
    width: 60px;
    height: 60px;
    border-radius: 50%;
    background: var(--accent);
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
  }

  .ctrl-play:disabled {
    opacity: 0.6;
  }

  .play-tri {
    width: 0;
    height: 0;
    border-style: solid;
    border-width: 11px 0 11px 18px;
    border-color: transparent transparent transparent #0d0d0d;
    margin-left: 3px;
  }

  .tune-listeners {
    font-family: "Share Tech Mono", monospace;
    font-size: 7px;
    letter-spacing: 1px;
    color: var(--text-muted);
    text-align: center;
    margin-bottom: 14px;
    text-transform: uppercase;
  }

  .lock-notice {
    margin: 0 14px;
    background: var(--bg-card);
    border: 1px solid var(--border-default);
    border-radius: 8px;
    padding: 10px 12px;
    display: flex;
    align-items: flex-start;
    gap: 9px;
  }

  .lock-notice-icon {
    font-size: 10px;
    line-height: 1.2;
    color: var(--accent);
    letter-spacing: 1px;
  }

  .lock-notice-title {
    font-family: "Share Tech Mono", monospace;
    font-size: 8px;
    letter-spacing: 1px;
    color: var(--accent);
    margin-bottom: 2px;
    text-transform: uppercase;
  }

  .lock-notice-sub {
    font-size: 10px;
    color: var(--text-muted);
  }

  @media (max-height: 760px) {
    .tune-screen {
      align-items: flex-start;
      justify-content: flex-start;
      padding-top: 12px;
    }
  }

  @keyframes blink {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.2;
    }
  }

  @keyframes wave {
    from {
      transform: scaleY(0.15);
      opacity: 0.3;
    }
    to {
      transform: scaleY(1);
      opacity: 0.9;
    }
  }

  @media (min-width: 1024px) {
    .desktop-view {
      display: block;
    }

    .mobile-view {
      display: none;
    }
  }
</style>
