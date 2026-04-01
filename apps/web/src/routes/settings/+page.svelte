<script lang="ts">
  import { createChannel, fetchOwnedChannels, login, register } from "$lib/api/client";
  import {
    ownedChannel,
    setOwnedChannels,
  } from "$lib/stores/channel";
  import {
    activeLocation,
    clearSinggah,
    isPro,
    manualLocation,
    singgah,
  } from "$lib/stores/location";
  import { clearSession, setSession, user } from "$lib/stores/user";

  let authMode: "login" | "register" = "login";
  let username = "";
  let password = "";
  let authBusy = false;
  let authError: string | null = null;
  let authSuccess: string | null = null;

  let channelName = "";
  let channelFrequency = "98.7";
  let channelRadius = "15000";
  let channelBusy = false;
  let channelError: string | null = null;
  let channelSuccess: string | null = null;

  const singgahPresets = [
    { label: "Jakarta", lat: -6.2088, lng: 106.8456 },
    { label: "Bandung", lat: -6.9175, lng: 107.6191 },
    { label: "Yogyakarta", lat: -7.7971, lng: 110.3708 },
  ];

  async function submitAuth() {
    authBusy = true;
    authError = null;
    authSuccess = null;

    try {
      const session =
        authMode === "login"
          ? await login({ username, password })
          : await register({ username, password });

      setSession(session);
      setOwnedChannels(await fetchOwnedChannels().catch(() => []));
      authSuccess = authMode === "login" ? "Signed in." : "Account created and signed in.";
      password = "";
    } catch (error) {
      authError = error instanceof Error ? error.message : "Authentication failed.";
    } finally {
      authBusy = false;
    }
  }

  async function submitChannel() {
    if (!$activeLocation) {
      channelError = "A current location is required to create a channel.";
      return;
    }

    channelBusy = true;
    channelError = null;
    channelSuccess = null;

    try {
      await createChannel({
        name: channelName,
        frequency: Number(channelFrequency),
        lat: $activeLocation.lat,
        lng: $activeLocation.lng,
        radiusM: Number(channelRadius),
      });

      setOwnedChannels(await fetchOwnedChannels());
      channelSuccess = "Channel created.";
      channelName = "";
    } catch (error) {
      channelError = error instanceof Error ? error.message : "Could not create channel.";
    } finally {
      channelBusy = false;
    }
  }

  function handleSignOut() {
    clearSession();
    setOwnedChannels([]);
    authSuccess = "Signed out.";
  }
</script>

<div class:guest-layout={!$user} class="page-shell settings-shell">
  <section class="panel account-card">
    <span class="section-label">Account</span>

    {#if $user}
      <div class="meta-row">
        <span class="meta-label">Username</span>
        <span class="meta-value mono">@{$user.username}</span>
      </div>
      <div class="meta-row">
        <span class="meta-label">Plan</span>
        <span class="meta-value mono">{$isPro ? "Pro" : "Free"}</span>
      </div>

      <button class="danger-button signout" type="button" on:click={handleSignOut}>
        Sign Out
      </button>
    {:else}
      <div class="auth-switch">
        <button
          class:active={authMode === "login"}
          class="ghost-button mini-button"
          type="button"
          on:click={() => (authMode = "login")}
        >
          Login
        </button>
        <button
          class:active={authMode === "register"}
          class="ghost-button mini-button"
          type="button"
          on:click={() => (authMode = "register")}
        >
          Register
        </button>
      </div>

      <div class="field-grid">
        <div class="field">
          <label>
            Username
            <input bind:value={username} placeholder="your handle" />
          </label>
        </div>
        <div class="field">
          <label>
            Password
            <input bind:value={password} type="password" placeholder="min 8 characters" />
          </label>
        </div>
      </div>

      <button class="primary-button" disabled={authBusy} type="button" on:click={submitAuth}>
        {authBusy ? "Working..." : authMode === "login" ? "Sign In" : "Create Account"}
      </button>
    {/if}

    {#if authError}
      <p class="status-copy">{authError}</p>
    {/if}

    {#if authSuccess}
      <p class="status-copy">{authSuccess}</p>
    {/if}
  </section>

  <section class="panel location-card">
    <span class="section-label">Location</span>

    <div class="meta-row">
      <span class="meta-label">Active Position</span>
      <span class="meta-value mono">
        {#if $activeLocation}
          {$activeLocation.lat.toFixed(4)}, {$activeLocation.lng.toFixed(4)}
        {:else}
          Waiting for GPS
        {/if}
      </span>
    </div>

    {#if $user}
      <div class="meta-row">
        <span class="meta-label">Singgah</span>
        <span class="meta-value mono">
          {#if $manualLocation}
            {$manualLocation.lat.toFixed(4)}, {$manualLocation.lng.toFixed(4)}
          {:else}
            Not set
          {/if}
        </span>
      </div>

      <div class="preset-grid">
        {#each singgahPresets as preset}
          <button class="ghost-button mini-button" type="button" on:click={() => singgah(preset)}>
            {preset.label}
          </button>
        {/each}
        <button class="ghost-button mini-button" type="button" on:click={clearSinggah}>
          Clear
        </button>
      </div>

      {#if !$isPro}
        <p class="status-copy">Singgah presets only take effect for Pro listeners.</p>
      {/if}
    {:else}
      <p class="status-copy">Sign in first to manage Singgah presets.</p>
    {/if}
  </section>

  <section class="panel channel-card">
    <span class="section-label">Your Channel</span>

    {#if $ownedChannel}
      <div class="meta-row">
        <span class="meta-label">Name</span>
        <span class="meta-value">{$ownedChannel.name}</span>
      </div>
      <div class="meta-row">
        <span class="meta-label">Frequency</span>
        <span class="meta-value mono">{$ownedChannel.frequency.toFixed(1)} FM</span>
      </div>
      <div class="meta-row">
        <span class="meta-label">Radius</span>
        <span class="meta-value mono">{Math.round($ownedChannel.radiusM / 1000)} km</span>
      </div>
      <div class="meta-row">
        <span class="meta-label">Status</span>
        <span class="meta-value mono">{$ownedChannel.status}</span>
      </div>
    {:else if $user}
      <div class="field-grid">
        <div class="field">
          <label>
            Channel Name
            <input bind:value={channelName} placeholder="Kopi Pagi FM" />
          </label>
        </div>

        <div class="two-col">
          <div class="field">
            <label>
              Frequency
              <input bind:value={channelFrequency} inputmode="decimal" placeholder="98.7" />
            </label>
          </div>

          <div class="field">
            <label>
              Radius (m)
              <input bind:value={channelRadius} inputmode="numeric" placeholder="15000" />
            </label>
          </div>
        </div>
      </div>

      <button class="primary-button" disabled={channelBusy} type="button" on:click={submitChannel}>
        {channelBusy ? "Creating..." : "Create Channel"}
      </button>
    {:else}
      <p class="status-copy">Sign in first, then create the station you want to broadcast from.</p>
    {/if}

    {#if channelError}
      <p class="status-copy">{channelError}</p>
    {/if}

    {#if channelSuccess}
      <p class="status-copy">{channelSuccess}</p>
    {/if}
  </section>
</div>

<style>
  .settings-shell {
    display: grid;
    gap: 18px;
    padding: 18px 16px 0;
  }

  .account-card,
  .location-card,
  .channel-card {
    padding: 20px;
    display: grid;
    gap: 16px;
  }

  .auth-switch,
  .preset-grid,
  .two-col {
    display: grid;
    gap: 10px;
  }

  .mini-button {
    min-height: 38px;
  }

  .mini-button.active {
    border-color: rgba(232, 200, 74, 0.4);
    color: var(--accent);
  }

  .signout {
    width: fit-content;
  }

  @media (min-width: 768px) {
    .auth-switch,
    .preset-grid,
    .two-col {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  @media (min-width: 1024px) {
    .settings-shell {
      grid-template-columns: repeat(3, minmax(0, 1fr));
      align-items: start;
      padding: 24px;
    }

    .settings-shell.guest-layout {
      display: flex;
      flex-direction: column;
      justify-content: center;
      align-items: center;
      min-height: calc(100vh - 56px - var(--sat) - 28px);
    }

    .settings-shell.guest-layout > section {
      width: min(100%, 720px);
    }
  }
</style>
