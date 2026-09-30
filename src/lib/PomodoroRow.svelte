<script lang="ts">
  import { t } from "../i18n/locale.svelte"
  import { formatClock, progress, stage } from "../store/pomodoro"
  import { pomodoro, pomodoroPanel, togglePomodoroPanel } from "../store/pomodoro.svelte"

  // Pinned under the workspace tree (outside its scroll area) while a session
  // exists, styled like a workspace row so the countdown reads as part of the
  // list but never scrolls away. Clicking it opens the same panel as the
  // footer button.
  let { collapsed }: { collapsed: boolean } = $props()

  let anchor = $state<HTMLButtonElement>()
  const clock = $derived(formatClock(pomodoro.remaining))
  const p = $derived(progress(pomodoro))
  const status = $derived(pomodoro.status)
  // Border colour steps from red toward green every quarter; green once done.
  const tone = $derived(stage(pomodoro))
  const note = $derived(
    status === "paused" ? t("pomodoro.paused") : status === "done" ? t("pomodoro.finished") : t("pomodoro.focus")
  )
</script>

{#if status !== "idle"}
  <div class="pin" class:collapsed>
    <button
      bind:this={anchor}
      class="row {status} s{tone}"
      class:on={pomodoroPanel.open}
      title={`${t("pomodoro.title")} · ${note} · ${clock}`}
      aria-label={`${t("pomodoro.title")} ${note} ${clock}`}
      aria-expanded={pomodoroPanel.open}
      onpointerdown={(e) => e.stopPropagation()}
      onclick={() => togglePomodoroPanel(anchor)}
    >
      {#if collapsed}
        <span class="clock">{clock}</span>
      {:else}
        <svg class="glass" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M6 3h12M6 21h12"></path><path d="M7 3c0 5 5 6 5 9s-5 4-5 9M17 3c0 5-5 6-5 9s5 4 5 9"></path></svg>
        <span class="name">{t("pomodoro.title")}</span>
        <span class="note">{note}</span>
        <span class="clock">{clock}</span>
      {/if}
      <!-- Progress border: traced clockwise from the top-left as time drains,
           closing into a full green outline when the session completes. -->
      <svg class="ring" aria-hidden="true">
        <rect class="track" pathLength="100"></rect>
        <rect class="trace" pathLength="100" stroke-dasharray="{status === 'done' ? 100 : p * 100} 100"></rect>
      </svg>
    </button>
  </div>
{/if}

<style>
  .pin {
    padding: 6px 8px 8px;
    border-top: 1px solid var(--border);
    animation: slide-up 320ms cubic-bezier(0.2, 0.9, 0.3, 1.15);
  }
  .pin.collapsed {
    padding: 6px;
  }
  .row {
    --tone: #e5534b;
    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    min-height: 40px;
    padding: 8px 10px;
    overflow: hidden;
    border: 0;
    border-radius: 7px;
    background: var(--bg-elevated);
    color: var(--text-1);
    font: inherit;
    font-size: calc(12.5px * var(--font-scale, 1));
    font-weight: 500;
    text-align: left;
    cursor: pointer;
  }
  .row.s1 {
    --tone: #ec8a4a;
  }
  .row.s2 {
    --tone: #e0b43a;
  }
  .row.s3 {
    --tone: #9cc94f;
  }
  .row.s4 {
    --tone: var(--ok);
  }
  :global([data-theme="light"]) .row.s0 {
    --tone: #c9372f;
  }
  :global([data-theme="light"]) .row.s1 {
    --tone: #c8621f;
  }
  :global([data-theme="light"]) .row.s2 {
    --tone: #a47c0c;
  }
  :global([data-theme="light"]) .row.s3 {
    --tone: #5e8f1c;
  }
  .row.done {
    animation: complete 1.2s ease-out;
  }
  .row:hover,
  .row.on {
    background: color-mix(in srgb, var(--tone) 10%, var(--bg-elevated));
  }
  .collapsed .row {
    justify-content: center;
    padding: 5px 0;
  }
  .glass {
    flex-shrink: 0;
    color: var(--tone);
  }
  .row.running .glass {
    animation: tip 2.4s ease-in-out infinite;
  }
  .name {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .note {
    flex: 1;
    overflow: hidden;
    color: var(--text-3);
    font-size: calc(11px * var(--font-scale, 1));
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .clock {
    flex-shrink: 0;
    color: var(--tone);
    font-family: var(--font-mono);
    font-size: calc(12px * var(--font-scale, 1));
    font-variant-numeric: tabular-nums;
  }
  .collapsed .clock {
    font-size: calc(10px * var(--font-scale, 1));
  }
  .row.paused .clock {
    animation: blink 1.4s ease-in-out infinite;
  }
  .ring {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    overflow: visible;
    pointer-events: none;
  }
  .ring rect {
    x: 0.75px;
    y: 0.75px;
    width: calc(100% - 1.5px);
    height: calc(100% - 1.5px);
    rx: 7px;
    fill: none;
    stroke-width: 1.5px;
  }
  .track {
    stroke: var(--border-strong);
  }
  .trace {
    stroke: var(--tone);
    stroke-linecap: round;
    transition: stroke-dasharray 250ms linear, stroke 600ms ease;
  }
  @keyframes slide-up {
    from { opacity: 0; transform: translateY(10px); }
  }
  @keyframes tip {
    0%, 80% { transform: rotate(0); }
    90%, 100% { transform: rotate(180deg); }
  }
  @keyframes complete {
    0% { box-shadow: 0 0 0 0 color-mix(in srgb, var(--ok) 55%, transparent); }
    100% { box-shadow: 0 0 0 10px transparent; }
  }
  @keyframes blink {
    50% { opacity: 0.35; }
  }
  @media (prefers-reduced-motion: reduce) {
    .pin, .glass, .clock, .row {
      animation: none !important;
    }
  }
</style>
