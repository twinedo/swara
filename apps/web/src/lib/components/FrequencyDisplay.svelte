<script lang="ts">
  import { formatFrequency } from "$lib/utils/format";

  export let frequency: number | null = null;
  export let size: "desktop" | "mobile" = "desktop";
</script>

<div class:desktop={size === "desktop"} class:mobile={size === "mobile"} class="frequency-display panel">
  <div class="sweep"></div>
  <div class="label mono">Tuned Frequency</div>
  <div class="value-row">
    <span class="value">{formatFrequency(frequency)}</span>
    <span class="suffix mono">FM</span>
  </div>
</div>

<style>
  .frequency-display {
    position: relative;
    overflow: hidden;
    padding: 22px 24px 18px;
    border-radius: 22px;
  }

  .label {
    position: relative;
    z-index: 1;
    font-size: 10px;
    letter-spacing: 2px;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .value-row {
    position: relative;
    z-index: 1;
    display: flex;
    align-items: baseline;
    gap: 10px;
  }

  .value {
    font-family: "Bebas Neue", sans-serif;
    line-height: 1;
    color: var(--accent);
    letter-spacing: 2px;
  }

  .suffix {
    font-size: 14px;
    letter-spacing: 2px;
    color: var(--text-muted);
  }

  .frequency-display.desktop .value {
    font-size: 68px;
  }

  .frequency-display.mobile .value {
    font-size: 52px;
  }

  .sweep {
    position: absolute;
    inset: 0 auto auto 0;
    width: 55%;
    height: 1px;
    background: linear-gradient(90deg, transparent, rgba(232, 200, 74, 0.7), transparent);
    animation: freq-sweep 4.5s linear infinite;
  }
</style>
