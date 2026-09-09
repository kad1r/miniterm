<script lang="ts">
  import { logoKeyFor } from "../store/ailogo"

  let { command, name }: { command: string; name: string } = $props()

  const key = $derived(logoKeyFor(command))

  // Anthropic's mark is a burst of tapered rays. Drawing it from a rotated
  // template keeps the geometry in one place instead of ten near-identical
  // path strings.
  const RAYS = [0, 36, 72, 108, 144]
</script>

{#if key === "claude"}
  <svg viewBox="0 0 24 24" aria-hidden="true">
    <g fill="var(--claude)">
      {#each RAYS as angle (angle)}
        <path d="M11.1 2.2h1.8l.5 9.8-1.4 1-1.4-1z" transform="rotate({angle} 12 12)" />
        <path d="M11.1 21.8h1.8l.5-9.8-1.4-1-1.4 1z" transform="rotate({angle} 12 12)" />
      {/each}
    </g>
  </svg>
{:else if key === "gemini"}
  <svg viewBox="0 0 24 24" aria-hidden="true">
    <!-- Four-point star with concave sides, on Google's blue→purple ramp. -->
    <defs>
      <linearGradient id="gemini-ramp" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0%" stop-color="#4285f4" />
        <stop offset="55%" stop-color="#9b72cb" />
        <stop offset="100%" stop-color="#d96570" />
      </linearGradient>
    </defs>
    <path
      fill="url(#gemini-ramp)"
      d="M12 2c0 5.5 4.5 10 10 10-5.5 0-10 4.5-10 10 0-5.5-4.5-10-10-10C7.5 12 12 7.5 12 2z"
    />
  </svg>
{:else}
  <!-- Custom tools get their initial rather than a mark we would have to invent. -->
  <span class="fallback">{name.slice(0, 1).toUpperCase()}</span>
{/if}

<style>
  svg,
  .fallback {
    flex: none;
    width: 18px;
    height: 18px;
  }
  .fallback {
    display: grid;
    place-items: center;
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-dim);
    font-size: calc(10px * var(--font-scale, 1));
    font-weight: 600;
  }
</style>
