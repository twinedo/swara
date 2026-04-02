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
  import { restoreSession } from "$lib/stores/user";
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
    { href: "/", label: "Tune" },
    { href: "/nearby", label: "Nearby" },
    { href: "/broadcast", label: "Broadcast" },
    { href: "/settings", label: "Settings" },
  ];
</script>

<svelte:head>
  <title>Swara</title>
</svelte:head>

<div class="shell">
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

  {#if $activeChannel && $page.url.pathname !== "/"}
    <div class="now-playing-wrap">
      <NowPlaying
        channel={$activeChannel}
        playing={$playbackState === "playing"}
        currentShow={currentShow}
        on:click={handleStopListening}
      />
    </div>
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
    .topbar {
      display: block;
    }

    .desktop-nav {
      display: flex;
    }

    .main {
      padding-bottom: 28px;
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
