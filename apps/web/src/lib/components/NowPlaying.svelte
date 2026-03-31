<script lang="ts">
  import type { Channel } from "@swara/types";

  import Waveform from "$lib/components/Waveform.svelte";
  import { formatFrequency, formatListeners } from "$lib/utils/format";

  export let channel: Channel;
  export let playing = false;
  export let currentShow: string | null = null;
</script>

<div class="now-playing panel">
  <div class="meta">
    <span class="section-label">Now Playing</span>
    <span class="freq mono">{formatFrequency(channel.frequency)} FM</span>
  </div>

  <div class="content">
    <div>
      <strong>{channel.name}</strong>
      <p>{currentShow ?? "Live community broadcast"}</p>
    </div>

    <div class="state">
      <Waveform active={playing} compact={true} />
      <span class="mono">{formatListeners(channel.listenerCount)} listeners</span>
    </div>
  </div>

  <button class="danger-button" type="button" on:click>Stop Listening</button>
</div>

<style>
  .now-playing {
    margin: 0 16px 16px;
    padding: 16px;
    border-radius: 22px;
  }

  .meta,
  .content {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }

  .content {
    margin: 12px 0 16px;
  }

  strong {
    display: block;
    font-size: 15px;
    margin-bottom: 4px;
  }

  p,
  .freq,
  .state {
    margin: 0;
    color: var(--text-muted);
  }

  .state {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 6px;
    font-size: 10px;
    letter-spacing: 1px;
    text-transform: uppercase;
  }

  @media (min-width: 1024px) {
    .now-playing {
      margin: 0;
    }
  }
</style>
