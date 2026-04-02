<script lang="ts">
  import type { Channel } from "@swara/types";
  import { get } from "svelte/store";

  import {
    fetchNearbyChannels,
    fetchSchedule,
    startListening,
  } from "$lib/api/client";
  import { joinAsListener, leaveRoom, resumeListenerAudio, setListenerVolume } from "$lib/livekit";
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
  import { formatDistance, formatFrequency, getCurrentShow } from "$lib/utils/format";

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
  let volumeLevel = 72;

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

    if (get(activeChannel)?.id === channel.id && get(playbackState) === "paused") {
      await resumeListenerAudio(get(listenerRoom));
      playbackState.set("playing");
      setPlaybackState("playing");
      return;
    }

    selectChannel(channel);

    if (channel.status !== "live") {
      playbackState.set("idle");
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
    if (featuredChannel) {
      void tune(featuredChannel);
    }
  }

  function handleVolumeInput(event: Event): void {
    const target = event.currentTarget as HTMLInputElement;
    volumeLevel = Number(target.value);
    setListenerVolume(volumeLevel / 100);
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
      activeSchedule.set([]);
      void loadSchedule($activeChannel.id);
    }
  } else {
    activeSchedule.set([]);
  }

  $: currentShow = getCurrentShow($activeSchedule);
  $: displayChannels = $nearbyChannels;
  $: hasNearbyChannels = displayChannels.length > 0;
  $: featuredChannel = displayChannels.find((channel) => channel.status === "live") ?? null;
  $: previewChannel = featuredChannel ?? displayChannels[0] ?? null;
  $: activeDesktopChannel = $activeChannel;
  $: mapChannels = displayChannels.slice(0, 4);
  $: scheduleRows = $activeSchedule;
  $: primaryLabel = primaryActionLabel($playbackState, featuredChannel);
  $: isDesktopPlaying = $playbackState === "playing" && !!activeDesktopChannel;
  $: isMobilePlaying = !!featuredChannel && $playbackState === "playing" && $activeChannel?.id === featuredChannel.id;
  $: isMobilePaused = !!featuredChannel && $playbackState === "paused" && $activeChannel?.id === featuredChannel.id;
  $: nowPlayingTitle = currentShow?.showName
    ?? (activeDesktopChannel ? `${activeDesktopChannel.name} Live Feed` : "Choose a nearby station");
  $: nowPlayingBy = currentShow?.hostName
    ? `by ${currentShow.hostName}`
    : activeDesktopChannel
      ? `${activeDesktopChannel.owner.username} - ${channelDistance(activeDesktopChannel)} away`
      : "Location access finds real stations around you";
  $: emptyScheduleCopy = $activeChannel
    ? "No schedule published for this channel yet."
    : "Tune a nearby station to see its schedule.";
</script>

<div class="desktop-view">
  <section class="desktop-wrap">
    <div class="app-shell">
      <div class="left-col">
        <div class="tuned-card">
          {#if activeDesktopChannel}
            <div class="card-label">NOW TUNED</div>
            <div class="tuned-freq-row">
              <div class="tuned-freq">{formatFrequency(activeDesktopChannel.frequency)}</div>
              <div class="tuned-fm">FM</div>
            </div>
            <div class="tuned-meta">
              <span>REGION <b>{activeDesktopChannel.owner.username}</b></span>
              <span>RADIUS <b>{Math.round($coverageRadius / 1000)} km</b></span>
              <span>LISTENERS <b>{activeDesktopChannel.listenerCount}</b></span>
            </div>
            <div class="waveform" aria-hidden="true">
              {#each desktopWave as bar, index}
                <div class="wv" style={waveStyle(bar, index)}></div>
              {/each}
            </div>
          {:else}
            <div class="card-label">READY TO TUNE</div>
            <div class="tuned-freq-row idle">
              <div class="tuned-freq placeholder">--.-</div>
              <div class="tuned-fm">FM</div>
            </div>
            <div class="tuned-meta">
              <span>REGION <b>{$activeLocation ? "Area Locked" : "Waiting for GPS"}</b></span>
              <span>RADIUS <b>{Math.round($coverageRadius / 1000)} km</b></span>
              <span>STATIONS <b>{displayChannels.length}</b></span>
            </div>
            <div class="idle-copy">
              Pick a nearby station to start listening. Nothing is playing yet.
            </div>
            <div class="idle-actions">
              <a class="btn-share" href="/nearby">BROWSE</a>
              <button class="btn-tune" type="button" disabled={!featuredChannel} on:click={tunePrimary}>
                TUNE FEATURED
              </button>
            </div>
          {/if}
        </div>

        <div class="channels-label">NEARBY CHANNELS</div>

        {#if isLoading}
          <p class="status-copy">Scanning your area for nearby stations...</p>
        {:else if !hasNearbyChannels}
          <p class="status-copy">
            No nearby stations found yet. Create one in Broadcast or Settings, or move your search radius.
          </p>
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
          {#if !hasNearbyChannels}
            <p class="status-copy">Map markers appear after the API returns real nearby channels.</p>
          {/if}
        </div>

        <div class="np-card">
          {#if activeDesktopChannel}
            <div class="np-tag">NOW PLAYING</div>
            <div class="np-title">{nowPlayingTitle}</div>
            <div class="np-by">{nowPlayingBy}</div>
            <div class="np-btns">
              <button class="btn-tune" type="button" disabled={!featuredChannel} on:click={tunePrimary}>
                {primaryLabel}
              </button>
              <button class="btn-share" type="button">SHARE</button>
            </div>
            <div class="vol-row">
              <div class="vol-lbl">VOL</div>
              <input
                type="range"
                min="0"
                max="100"
                step="1"
                value={volumeLevel}
                class="vol-slider"
                style={`--volume-level:${volumeLevel}%`}
                aria-label="Volume"
                on:input={handleVolumeInput}
              />
            </div>
          {:else}
            <div class="np-tag idle-tag">LISTENING DECK</div>
            <div class="np-title idle-title">Choose a station to go live with the room audio feed.</div>
            <div class="np-by">
              Nearby stations stay browseable until you explicitly tune one. Broadcast and settings are available from the top menu.
            </div>
            <div class="np-btns">
              <button class="btn-tune" type="button" disabled={!featuredChannel} on:click={tunePrimary}>
                {featuredChannel ? `TUNE ${formatFrequency(featuredChannel.frequency)}` : "TUNE IN"}
              </button>
              <a class="btn-share link-button" href="/broadcast">BROADCAST</a>
            </div>
          {/if}
        </div>

        <div>
          <div class="sched-label">TODAY'S SCHEDULE</div>
          {#if scheduleRows.length > 0}
            {#each scheduleRows.slice(0, 4) as entry}
              <div class="sched-row">
                <div class="sched-time">{entry.startTime}</div>
                <div>
                  <div class="sched-show">{entry.showName}</div>
                  <div class="sched-host">{entry.hostName ?? "Community hosted"}</div>
                </div>
              </div>
            {/each}
          {:else}
            <p class="status-copy">{emptyScheduleCopy}</p>
          {/if}
        </div>
      </div>
    </div>
  </section>
</div>

<div class="mobile-view">
  <section class="mobile-shell">
    <div class="tune-mobile-header">
      <div class="tune-np-label">
        NOW PLAYING
        <div class="live-badge"><div class="live-badge-dot"></div>LIVE</div>
      </div>
    </div>

    <div class="tune-screen">
      <div class="tune-stack">
        <div class="tune-hero">
          <div class="tune-freq-big">{previewChannel ? formatFrequency(previewChannel.frequency) : "--.-"}</div>
          <div class="tune-fm">FM</div>
        </div>
        <div class="tune-name">{previewChannel?.name ?? "No live stations nearby"}</div>
        <div class="tune-sub">
          {previewChannel
            ? `${channelLocation(previewChannel)} away`
            : "Enable location and create a live channel to test audio."}
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
            aria-label={isMobilePlaying ? "Stop listening" : isMobilePaused ? "Resume listening" : "Tune in"}
            disabled={!featuredChannel}
            on:click={tunePrimary}
          >
            {#if isMobilePlaying}
              <div class="pause-bars" aria-hidden="true">
                <span></span>
                <span></span>
              </div>
            {:else}
              <div class:resume-tri={isMobilePaused} class="play-tri"></div>
            {/if}
          </button>
          <div class="ctrl-heart">FAV</div>
        </div>
        <div class="tune-listeners">
          {featuredChannel
            ? `${(featuredChannel.listenerCount ?? 0).toString()} people tuned in nearby`
            : previewChannel
              ? "This nearby station is offline right now"
              : "No live listeners nearby yet"}
        </div>

        <div class="lock-notice">
          <div class="lock-notice-icon">LOCK</div>
          <div>
            <div class="lock-notice-title">{errorMessage ? "Playback notice" : featuredChannel ? "Lock screen controls active" : "Station offline"}</div>
            <div class="lock-notice-sub">
              {errorMessage ?? (featuredChannel ? "Audio keeps playing in background" : "Start broadcasting on the desktop first, then tune in here.")}
            </div>
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
    border-radius: 14px;
    border: 2px solid #2a2a2a;
    overflow: hidden;
  }

  .app-shell {
    background: var(--bg-primary);
    display: grid;
    grid-template-columns: 1fr 1fr;
    min-height: 580px;
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

  .tuned-freq-row.idle {
    margin-bottom: 12px;
  }

  .tuned-freq {
    font-family: "Bebas Neue", sans-serif;
    font-size: 64px;
    color: var(--accent);
    line-height: 1;
  }

  .tuned-freq.placeholder {
    color: rgba(232, 200, 74, 0.55);
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

  .idle-copy {
    font-size: 14px;
    color: var(--text-primary);
    line-height: 1.5;
    max-width: 34ch;
    margin-bottom: 16px;
  }

  .idle-actions {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
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

  .idle-tag {
    color: var(--accent);
  }

  .idle-title {
    font-size: 24px;
    max-width: 14ch;
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

  .link-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    text-decoration: none;
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

  .vol-slider {
    flex: 1;
    appearance: none;
    height: 3px;
    border-radius: 999px;
    outline: none;
    background: linear-gradient(
      90deg,
      var(--text-primary) 0%,
      var(--text-primary) var(--volume-level),
      var(--border-default) var(--volume-level),
      var(--border-default) 100%
    );
  }

  .vol-slider::-webkit-slider-thumb {
    appearance: none;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--text-primary);
    border: 2px solid var(--bg-primary);
    cursor: pointer;
  }

  .vol-slider::-moz-range-thumb {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--text-primary);
    border: 2px solid var(--bg-primary);
    cursor: pointer;
  }

  .vol-slider::-moz-range-track {
    height: 3px;
    border-radius: 999px;
    background: transparent;
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
    height: calc(100svh - 104px - var(--sab));
    display: block;
  }

  .mobile-shell {
    height: 100%;
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
    padding: 0;
  }

  .tune-stack {
    width: 100%;
    max-width: 360px;
    margin: 0 auto;
    min-height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: center;
  }

  .tune-mobile-header {
    position: sticky;
    top: 0;
    z-index: 10;
    background: rgba(13, 13, 13, 0.92);
    backdrop-filter: blur(18px);
  }

  .tune-np-label {
    font-family: "Share Tech Mono", monospace;
    font-size: 12px;
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
    font-size: 12px;
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
    font-size: 10px;
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

  .play-tri.resume-tri {
    transform: scale(0.9);
  }

  .pause-bars {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .pause-bars span {
    width: 7px;
    height: 22px;
    border-radius: 999px;
    background: #0d0d0d;
  }

  .tune-listeners {
    font-family: "Share Tech Mono", monospace;
    font-size: 10px;
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
    font-size: 12px;
    letter-spacing: 1px;
    color: var(--accent);
    margin-bottom: 2px;
    text-transform: uppercase;
  }

  .lock-notice-sub {
    font-size: 11px;
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
