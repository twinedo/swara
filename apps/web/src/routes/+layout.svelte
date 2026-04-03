<script lang="ts">
  import { afterNavigate, beforeNavigate } from "$app/navigation";
  import { page } from "$app/stores";
  import { onMount } from "svelte";
  import { get } from "svelte/store";

  import "../app.css";
  import { fetchOwnedChannels } from "$lib/api/client";
  import BottomNav from "$lib/components/BottomNav.svelte";
  import NowPlaying from "$lib/components/NowPlaying.svelte";
  import { leaveRoom, pauseListenerAudio, resumeListenerAudio } from "$lib/livekit";
  import { clearMediaSession, setPlaybackState } from "$lib/mediaSession";
  import {
    activeChannel,
    activeSchedule,
    listenerRoom,
    playbackState,
    resetListeningState,
    setOwnedChannels,
  } from "$lib/stores/channel";
  import { restoreSinggah, startLocationWatch } from "$lib/stores/location";
  import { isAuthenticated, restoreSession, user } from "$lib/stores/user";
  import { getCurrentShow } from "$lib/utils/format";

  async function hydrateOwnedChannel() {
    try {
      const channels = await fetchOwnedChannels();
      setOwnedChannels(channels);
    } catch {
      setOwnedChannels([]);
    }
  }

  onMount(() => {
    restoreSession();
    restoreSinggah();
    const stopWatching = startLocationWatch();
    void hydrateOwnedChannel();

    const handleVisibilityChange = () => {
      if (document.visibilityState === "visible") {
        void resumeSharedPlayback();
      }
    };

    const handlePageShow = () => {
      void resumeSharedPlayback();
    };

    document.addEventListener("visibilitychange", handleVisibilityChange);
    window.addEventListener("pageshow", handlePageShow);

    beforeNavigate(() => {
      pauseSharedPlayback();
    });

    afterNavigate(() => {
      void resumeSharedPlayback();
    });

    return () => {
      document.removeEventListener("visibilitychange", handleVisibilityChange);
      window.removeEventListener("pageshow", handlePageShow);
      stopWatching();
    };
  });

  async function handleStopListening() {
    await leaveRoom($listenerRoom);
    resetListeningState();
    activeChannel.set(null);
    clearMediaSession();
    setPlaybackState("none");
  }

  async function resumeSharedPlayback() {
    if (!get(listenerRoom) || get(playbackState) !== "playing") {
      return;
    }

    await resumeListenerAudio(get(listenerRoom));
    setPlaybackState("playing");
  }

  function pauseSharedPlayback() {
    if (!get(listenerRoom) || get(playbackState) !== "playing") {
      return;
    }

    pauseListenerAudio(get(listenerRoom));
    setPlaybackState("paused");
  }

  $: currentShow = getCurrentShow($activeSchedule)?.showName ?? null;
  const navLinks = [
    { href: "/", label: "Tune", detail: "Live listening deck" },
    { href: "/nearby", label: "Nearby", detail: "Scan your area" },
    { href: "/broadcast", label: "Broadcast", detail: "Go on air" },
    { href: "/settings", label: "Settings", detail: "Account and station" },
  ];

  $: showPersistentPlayer = !!$activeChannel && $page.url.pathname !== "/";
</script>

<svelte:head>
  <title>Swara</title>
</svelte:head>

<div class:dashboard-shell={$isAuthenticated} class="shell">
  {#if $isAuthenticated}
    <div class="dashboard-grid">
      <aside class="sidebar">
        <div class="sidebar-card">
          <a class="brand sidebar-brand" href="/">SWARA <span>V1</span></a>
          <p class="sidebar-copy">
            Your radio dashboard keeps the main actions pinned on the left and the live workspace in
            the center.
          </p>
        </div>

        <nav aria-label="Dashboard" class="sidebar-nav">
          {#each navLinks as link}
            <a class:active={$page.url.pathname === link.href} class="sidebar-link" href={link.href}>
              <span class="sidebar-link-label">{link.label}</span>
              <span class="sidebar-link-detail">{link.detail}</span>
            </a>
          {/each}
        </nav>

        <div class="sidebar-status">
          <div class="sidebar-status-label">Signed In</div>
          <div class="sidebar-status-value mono">@{$user?.username ?? "listener"}</div>
          <p class="sidebar-status-copy">
            Dashboard mode is active. Use Settings to manage your account and station.
          </p>
        </div>
      </aside>

      <div class="dashboard-stage">
        <main class="main dashboard-main">
          <div class="dashboard-frame">
            <slot />
          </div>
        </main>

        {#if showPersistentPlayer && $activeChannel}
          <div class="dashboard-player">
            <div class="dashboard-frame">
              <NowPlaying
                channel={$activeChannel}
                playing={$playbackState === "playing"}
                currentShow={currentShow}
                on:click={handleStopListening}
              />
            </div>
          </div>
        {/if}
      </div>
    </div>
  {:else}
    <header class="topbar">
      <div class="topbar-inner">
        <a class="brand" href="/">SWARA <span>V1</span></a>

        <nav class="desktop-nav">
          {#each navLinks as link}
            <a class:active={$page.url.pathname === link.href} href={link.href}>{link.label}</a>
          {/each}
        </nav>
      </div>
    </header>

    <main class="main">
      <slot />
    </main>

    {#if showPersistentPlayer && $activeChannel}
      <div class="now-playing-wrap">
        <NowPlaying
          channel={$activeChannel}
          playing={$playbackState === "playing"}
          currentShow={currentShow}
          on:click={handleStopListening}
        />
      </div>
    {/if}
  {/if}

  <div class="mobile-nav">
    <BottomNav />
  </div>
</div>

<style>
  :global(body) {
    min-height: 100vh;
  }

  .shell {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
  }

  .dashboard-shell {
    background:
      radial-gradient(circle at top left, rgba(232, 200, 74, 0.08), transparent 24%),
      linear-gradient(180deg, #111 0%, var(--bg-primary) 28%);
  }

  .topbar {
    display: none;
    position: sticky;
    top: 0;
    z-index: 20;
    padding-top: var(--sat);
    backdrop-filter: blur(18px);
    background: rgba(13, 13, 13, 0.86);
    border-bottom: 1px solid rgba(255, 255, 255, 0.04);
  }

  .topbar-inner {
    min-height: 56px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 0 max(18px, var(--sal)) 0 max(18px, var(--sar));
  }

  .brand {
    font-family: "Bebas Neue", sans-serif;
    font-size: 28px;
    letter-spacing: 4px;
    color: var(--accent);
    text-decoration: none;
  }

  .brand span {
    font-size: 14px;
    color: var(--text-muted);
    letter-spacing: 3px;
  }

  .desktop-nav {
    display: none;
    align-items: center;
    gap: 24px;
  }

  .desktop-nav a {
    font-family: "Share Tech Mono", monospace;
    font-size: 10px;
    letter-spacing: 2px;
    text-transform: uppercase;
    text-decoration: none;
    color: var(--text-muted);
  }

  .desktop-nav a.active {
    color: var(--accent);
  }

  .main {
    flex: 1;
    padding-bottom: calc(104px + var(--sab));
  }

  .dashboard-grid {
    flex: 1;
    min-height: 100vh;
  }

  .sidebar {
    display: none;
  }

  .sidebar-card,
  .sidebar-status {
    background: linear-gradient(180deg, rgba(25, 25, 25, 0.92), rgba(15, 15, 15, 0.92));
    border: 1px solid var(--border-default);
    border-radius: 20px;
    box-shadow: var(--shadow-panel);
  }

  .sidebar-card {
    padding: 22px 20px;
  }

  .sidebar-brand {
    display: inline-block;
    margin-bottom: 12px;
  }

  .sidebar-copy {
    margin: 0;
    color: var(--text-muted);
    line-height: 1.6;
    font-size: 14px;
  }

  .sidebar-nav {
    display: grid;
    gap: 10px;
  }

  .sidebar-link {
    display: grid;
    gap: 5px;
    padding: 14px 16px;
    border-radius: 18px;
    text-decoration: none;
    border: 1px solid rgba(255, 255, 255, 0.04);
    background: rgba(255, 255, 255, 0.02);
    transition:
      background 160ms ease,
      border-color 160ms ease,
      transform 160ms ease;
  }

  .sidebar-link:hover {
    transform: translateX(2px);
    border-color: rgba(232, 200, 74, 0.22);
    background: rgba(232, 200, 74, 0.06);
  }

  .sidebar-link.active {
    border-color: rgba(232, 200, 74, 0.34);
    background: linear-gradient(180deg, rgba(232, 200, 74, 0.14), rgba(232, 200, 74, 0.06));
  }

  .sidebar-link-label {
    font-family: "Share Tech Mono", monospace;
    font-size: 12px;
    letter-spacing: 2px;
    text-transform: uppercase;
    color: var(--text-primary);
  }

  .sidebar-link-detail {
    color: var(--text-muted);
    font-size: 13px;
    line-height: 1.4;
  }

  .sidebar-status {
    margin-top: auto;
    padding: 18px 20px;
  }

  .sidebar-status-label {
    margin-bottom: 8px;
    font-family: "Share Tech Mono", monospace;
    font-size: 10px;
    letter-spacing: 2px;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .sidebar-status-value {
    font-size: 18px;
    color: var(--accent);
  }

  .sidebar-status-copy {
    margin: 10px 0 0;
    color: var(--text-muted);
    line-height: 1.55;
    font-size: 13px;
  }

  .dashboard-stage {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
  }

  .dashboard-main {
    padding: 18px 0 calc(112px + var(--sab));
  }

  .dashboard-frame {
    width: min(1220px, calc(100vw - 32px));
    margin: 0 auto;
  }

  .dashboard-player {
    display: none;
  }

  .mobile-nav {
    position: fixed;
    inset: auto 0 0;
    z-index: 40;
  }

  .now-playing-wrap {
    position: fixed;
    inset: auto 0 calc(74px + var(--sab));
    z-index: 35;
  }

  @media (min-width: 1024px) {
    .dashboard-grid {
      display: grid;
      grid-template-columns: 300px minmax(0, 1fr);
    }

    .sidebar {
      position: sticky;
      top: 0;
      display: flex;
      flex-direction: column;
      gap: 18px;
      height: 100vh;
      padding: calc(24px + var(--sat)) 20px 24px max(20px, var(--sal));
      border-right: 1px solid rgba(255, 255, 255, 0.04);
      background:
        linear-gradient(180deg, rgba(10, 10, 10, 0.96), rgba(13, 13, 13, 0.92)),
        radial-gradient(circle at top left, rgba(232, 200, 74, 0.08), transparent 34%);
    }

    .topbar {
      display: block;
    }

    .desktop-nav {
      display: flex;
    }

    .main {
      padding-bottom: 28px;
    }

    .dashboard-main {
      padding: calc(28px + var(--sat)) 28px 12px;
    }

    .dashboard-frame {
      width: min(1220px, 100%);
    }

    .dashboard-player {
      display: block;
      position: sticky;
      bottom: 0;
      padding: 0 28px 24px;
      background: linear-gradient(180deg, rgba(13, 13, 13, 0), rgba(13, 13, 13, 0.92) 42%);
    }

    .mobile-nav {
      display: none;
    }

    .now-playing-wrap {
      position: sticky;
      bottom: 0;
      padding: 0 24px;
    }
  }
</style>
