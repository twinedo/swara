<script lang="ts">
  export let active = false;
  export let bars = 5;
  export let compact = false;

  const pattern = [18, 9, 14, 6, 16, 11, 20, 8];
  $: heights = Array.from({ length: bars }, (_, index) => pattern[index % pattern.length]);
</script>

<div class:compact class="waveform" aria-hidden="true">
  {#each heights as height, index}
    <span
      class:active
      class="bar"
      style={`--bar-height:${height}px; --delay:${index * 0.08}s;`}
    ></span>
  {/each}
</div>

<style>
  .waveform {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    min-height: 20px;
  }

  .waveform.compact {
    gap: 3px;
    min-height: 16px;
  }

  .bar {
    width: 4px;
    height: var(--bar-height);
    border-radius: 999px;
    background: rgba(232, 200, 74, 0.18);
    transform-origin: center;
  }

  .compact .bar {
    width: 3px;
  }

  .bar.active {
    background: var(--accent);
    animation: wave 420ms ease-in-out infinite alternate;
    animation-delay: var(--delay);
  }
</style>
