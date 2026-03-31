<script lang="ts">
  import { page } from "$app/stores";

  const items = [
    { href: "/", label: "Tune", icon: "📻" },
    { href: "/nearby", label: "Nearby", icon: "📡" },
    { href: "/broadcast", label: "Broadcast", icon: "🎙" },
    { href: "/settings", label: "Settings", icon: "⚙" },
  ];
</script>

<nav class="bottom-nav">
  {#each items as item}
    <a class:active={$page.url.pathname === item.href} href={item.href}>
      <span class="icon">{item.icon}</span>
      <span class="label">{item.label}</span>
    </a>
  {/each}
</nav>

<style>
  .bottom-nav {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 4px;
    background: rgba(15, 15, 15, 0.94);
    border-top: 3px solid var(--border-default);
    backdrop-filter: blur(18px);
    padding: 6px 0;

  }

  a {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    min-height: 56px;
    padding: 8px 0 6px;
    text-decoration: none;
    color: var(--text-muted);
  }

  a.active {
    color: var(--accent);
  }

  .icon {
    font-size: 20px;
    line-height: 1;
  }

  .label {
    font-family: "Share Tech Mono", monospace;
    font-size: 12px;
    letter-spacing: 1.8px;
    text-transform: uppercase;
  }
</style>
