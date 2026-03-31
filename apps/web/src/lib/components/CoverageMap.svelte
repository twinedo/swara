<script lang="ts">
  import type { Channel, LatLng } from "@swara/types";
  import { createEventDispatcher } from "svelte";

  import { formatFrequency } from "$lib/utils/format";

  export let center: LatLng | null = null;
  export let channels: Channel[] = [];
  export let radiusMetres = 15_000;
  export let allowSinggah = false;

  const dispatch = createEventDispatcher<{ singgah: LatLng }>();
  const viewCenter = 50;
  const viewRadius = 34;
  const metresPerDegreeLat = 111_320;

  function seedFrom(value: string): number {
    let seed = 0;

    for (const character of value) {
      seed = (seed * 31 + character.charCodeAt(0)) % 10_000;
    }

    return seed / 10_000;
  }

  function project(point: LatLng) {
    if (!center) {
      return { x: viewCenter, y: viewCenter };
    }

    const metresPerDegreeLng = metresPerDegreeLat * Math.cos((center.lat * Math.PI) / 180);
    const dx = (point.lng - center.lng) * metresPerDegreeLng;
    const dy = (point.lat - center.lat) * metresPerDegreeLat;
    const scale = viewRadius / radiusMetres;
    const clamped = Math.min(1, viewRadius / Math.max(Math.hypot(dx * scale, dy * scale), 0.0001));
    return {
      x: viewCenter + dx * scale * Math.min(clamped, 1),
      y: viewCenter - dy * scale * Math.min(clamped, 1),
    };
  }

  function buildCoveragePolygon(): string {
    const points: string[] = [];

    for (let index = 0; index < 64; index += 1) {
      const angle = (index / 64) * Math.PI * 2;
      const x = viewCenter + Math.cos(angle) * viewRadius;
      const y = viewCenter + Math.sin(angle) * viewRadius;
      points.push(`${x},${y}`);
    }

    return points.join(" ");
  }

  function handleMapClick(event: MouseEvent): void {
    if (!allowSinggah || !center) {
      return;
    }

    const target = event.currentTarget as SVGSVGElement;
    const bounds = target.getBoundingClientRect();
    const x = ((event.clientX - bounds.left) / bounds.width) * 100;
    const y = ((event.clientY - bounds.top) / bounds.height) * 100;
    const scale = radiusMetres / viewRadius;
    const metresPerDegreeLng = metresPerDegreeLat * Math.cos((center.lat * Math.PI) / 180);
    const dx = (x - viewCenter) * scale;
    const dy = (viewCenter - y) * scale;

    dispatch("singgah", {
      lat: center.lat + dy / metresPerDegreeLat,
      lng: center.lng + dx / metresPerDegreeLng,
    });
  }

  $: polygon = buildCoveragePolygon();
  $: pins = center
    ? channels.map((channel) => ({
        ...channel,
        point: project({
          lat:
            center.lat +
            (seedFrom(`${channel.id}:lat`) - 0.5) * (radiusMetres / metresPerDegreeLat) * 0.8,
          lng:
            center.lng +
            (seedFrom(`${channel.id}:lng`) - 0.5) *
              (radiusMetres /
                (metresPerDegreeLat * Math.max(Math.cos((center.lat * Math.PI) / 180), 0.2))) *
              0.8,
        }),
      }))
    : [];
</script>

<div class="map-card panel">
  <div class="header">
    <span class="section-label">Coverage Map</span>
    <span class="radius mono">{Math.round(radiusMetres / 1000)} km radius</span>
  </div>

  <button
    class:interactive={allowSinggah}
    class="map-trigger"
    type="button"
    aria-label={allowSinggah ? "Coverage map, tap to set Singgah" : "Coverage map"}
    disabled={!allowSinggah}
    on:click={handleMapClick}
  >
    <svg class="radar" viewBox="0 0 100 100" role="img" aria-hidden="true">
      <circle class="ring outer" cx="50" cy="50" r="38"></circle>
      <circle class="ring" cx="50" cy="50" r="28"></circle>
      <circle class="ring" cx="50" cy="50" r="18"></circle>
      <polygon class="coverage" points={polygon}></polygon>
      <circle class="center-dot" cx="50" cy="50" r="2.5"></circle>
      <text class="center-label" x="52" y="54">You</text>

      {#each pins as pin (pin.id)}
        <g transform={`translate(${pin.point.x} ${pin.point.y})`}>
          <circle class:live={pin.status === "live"} class="pin-dot" r="2.2"></circle>
          <text class="pin-label" x="0" y="-4">{formatFrequency(pin.frequency)}</text>
        </g>
      {/each}
    </svg>
  </button>

  {#if allowSinggah}
    <p class="help mono">Tap the radar to drop a Singgah point.</p>
  {/if}
</div>

<style>
  .map-card {
    padding: 18px;
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 18px;
  }

  .radius {
    font-size: 10px;
    letter-spacing: 1.6px;
    color: var(--text-muted);
    text-transform: uppercase;
  }

  .map-trigger {
    width: 100%;
    padding: 0;
    border: 0;
    background: transparent;
  }

  .map-trigger:disabled {
    cursor: default;
  }

  .radar {
    width: 100%;
    min-height: 240px;
    border-radius: 18px;
    background:
      radial-gradient(circle at center, rgba(16, 37, 14, 0.95), rgba(6, 12, 8, 0.98)),
      linear-gradient(180deg, rgba(255, 255, 255, 0.04), transparent);
    border: 1px solid rgba(97, 255, 151, 0.08);
  }

  .map-trigger.interactive .radar {
    cursor: crosshair;
  }

  .ring {
    fill: none;
    stroke: rgba(134, 255, 172, 0.16);
    stroke-width: 0.45;
  }

  .ring.outer {
    stroke-dasharray: 2 2;
  }

  .coverage {
    fill: rgba(232, 200, 74, 0.08);
    stroke: rgba(232, 200, 74, 0.42);
    stroke-dasharray: 2.5 2.5;
    stroke-width: 0.5;
  }

  .center-dot {
    fill: var(--accent);
    animation: pulse-accent 2.6s ease-in-out infinite;
  }

  .center-label,
  .pin-label {
    fill: var(--text-primary);
    font-size: 4.2px;
    font-family: "Share Tech Mono", monospace;
    letter-spacing: 0.4px;
  }

  .pin-dot {
    fill: var(--text-disabled);
    stroke: #08110a;
    stroke-width: 0.6;
  }

  .pin-dot.live {
    fill: var(--online);
  }

  .help {
    margin: 12px 0 0;
    font-size: 10px;
    letter-spacing: 1.2px;
    color: var(--text-muted);
    text-transform: uppercase;
  }
</style>
