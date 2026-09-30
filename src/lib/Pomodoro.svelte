<script lang="ts">
  import { t } from "../i18n/locale.svelte"
  import { formatClock, PRESETS_MIN, clampMinutes, progress } from "../store/pomodoro"
  import {
    pomodoro, pomodoroPanel, pausePomodoro, resetPomodoro, selectPomodoro, startPomodoro,
    togglePomodoroPanel,
  } from "../store/pomodoro.svelte"

  let anchor = $state<HTMLButtonElement>()
  let custom = $state("")
  // Bumped on every fresh start (not on resume) to replay the entrance effect.
  let sessionKey = $state(0)

  const live = $derived(pomodoro.status === "running" || pomodoro.status === "paused")
  const shown = $derived(pomodoro.status !== "idle")
  const clock = $derived(formatClock(pomodoro.remaining))
  const p = $derived(progress(pomodoro))
  const selectedMin = $derived(Math.round(pomodoro.duration / 60_000))

  // Hourglass geometry (viewBox 0 0 64 96). Top sand drains from y=14 down to
  // the neck at 46; bottom sand piles up from 86.
  const topY = $derived(14 + p * 32)
  const bottomY = $derived(86 - p * 30)

  function label(min: number): string {
    return min % 60 === 0 ? t("pomodoro.hours", { n: min / 60 }) : t("pomodoro.minutes", { n: min })
  }

  function pickCustom() {
    const n = Number.parseInt(custom, 10)
    if (Number.isNaN(n)) return
    selectPomodoro(clampMinutes(n))
    custom = String(clampMinutes(n))
  }

  function start() {
    if (pomodoro.status !== "paused") sessionKey++
    startPomodoro()
  }
</script>

<svelte:window
  onkeydown={(e) => pomodoroPanel.open && e.key === "Escape" && (pomodoroPanel.open = false)}
  onpointerdown={() => (pomodoroPanel.open = false)}
/>

<button
  bind:this={anchor}
  class="trigger"
  class:on={pomodoroPanel.open}
  class:live
  class:paused={pomodoro.status === "paused"}
  title={t("pomodoro.open")}
  aria-label={t("pomodoro.open")}
  aria-expanded={pomodoroPanel.open}
  onpointerdown={(e) => e.stopPropagation()}
  onclick={() => togglePomodoroPanel(anchor)}
>
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M6 3h12M6 21h12"></path><path d="M7 3c0 5 5 6 5 9s-5 4-5 9M17 3c0 5-5 6-5 9s5 4 5 9"></path></svg>
</button>

{#if pomodoroPanel.open}
  <div
    class="pop"
    style="left:{pomodoroPanel.left}px; bottom:{pomodoroPanel.bottom}px"
    role="dialog"
    aria-label={t("pomodoro.title")}
    tabindex="-1"
    onpointerdown={(e) => e.stopPropagation()}
  >
    <div class="head">
      <span class="title">{t("pomodoro.title")}</span>
      {#if pomodoro.status === "paused"}
        <span class="badge">{t("pomodoro.paused")}</span>
      {:else if pomodoro.status === "running"}
        <span class="badge focus">{t("pomodoro.focus")}</span>
      {/if}
    </div>

    {#if !shown}
      <p class="ask">{t("pomodoro.pick")}</p>
      <div class="presets">
        {#each PRESETS_MIN as min (min)}
          <button class="chip" class:sel={selectedMin === min} onclick={() => selectPomodoro(min)}>
            {label(min)}
          </button>
        {/each}
      </div>
      <label class="custom">
        <span>{t("pomodoro.custom")}</span>
        <input
          type="number"
          min="1"
          max="180"
          placeholder={String(selectedMin)}
          bind:value={custom}
          onchange={pickCustom}
          onkeydown={(e) => e.key === "Enter" && pickCustom()}
        />
      </label>
      <div class="preview">{clock}</div>
    {:else}
      {#key sessionKey}
        <div class="stage" class:running={pomodoro.status === "running"} class:done={pomodoro.status === "done"}>
          <div class="glow"></div>
          <svg class="glass" viewBox="0 0 64 96" width="72" height="108" aria-hidden="true">
            <defs>
              <clipPath id="pomo-bulb">
                <path d="M15 8 C15 30 29 40 29 48 C29 56 15 66 15 88 H49 C49 66 35 56 35 48 C35 40 49 30 49 8 Z"></path>
              </clipPath>
            </defs>
            <g clip-path="url(#pomo-bulb)">
              <rect class="sand" x="0" y={topY} width="64" height={Math.max(0, 47 - topY)}></rect>
              <rect class="sand" x="0" y={bottomY} width="64" height={88 - bottomY}></rect>
            </g>
            {#if pomodoro.status === "running"}
              <line class="stream" x1="32" y1="46" x2="32" y2={bottomY}></line>
            {/if}
            <path class="frame" d="M15 8 C15 30 29 40 29 48 C29 56 15 66 15 88 M49 8 C49 30 35 40 35 48 C35 56 49 66 49 88"></path>
            <rect class="plate" x="9" y="3" width="46" height="5" rx="2"></rect>
            <rect class="plate" x="9" y="88" width="46" height="5" rx="2"></rect>
          </svg>
          <div class="clock" aria-live="polite">{clock}</div>
          <div class="bar"><span style="width:{p * 100}%"></span></div>
        </div>
      {/key}
    {/if}

    <div class="actions">
      {#if pomodoro.status === "running"}
        <button class="btn primary" onclick={pausePomodoro}>{t("pomodoro.pause")}</button>
      {:else}
        <button class="btn primary" onclick={start}>
          {pomodoro.status === "paused" ? t("pomodoro.resume") : t("pomodoro.start")}
        </button>
      {/if}
      <button class="btn" onclick={resetPomodoro} disabled={pomodoro.status === "idle"}>
        {t("pomodoro.reset")}
      </button>
    </div>
  </div>
{/if}

<style>
  .trigger {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    flex-shrink: 0;
    min-width: 30px;
    height: 30px;
    padding: 0 7px;
    border: 1px solid transparent;
    border-radius: var(--radius);
    background: none;
    color: var(--text-2);
    font: inherit;
    cursor: pointer;
  }
  .trigger:hover {
    background: color-mix(in srgb, var(--text-1) 8%, transparent);
    color: var(--text-1);
  }
  .trigger.on {
    background: var(--bg-elevated);
    color: var(--accent);
  }
  .trigger.live {
    color: var(--accent);
    border-color: color-mix(in srgb, var(--accent) 30%, transparent);
    background: color-mix(in srgb, var(--accent) 10%, transparent);
  }
  .trigger.live.paused {
    color: var(--warn);
    border-color: color-mix(in srgb, var(--warn) 30%, transparent);
    background: color-mix(in srgb, var(--warn) 10%, transparent);
  }
  .trigger.live:not(.paused) svg {
    animation: tip 2.4s ease-in-out infinite;
  }

  .pop {
    position: fixed;
    z-index: 60;
    width: 244px;
    padding: 14px;
    background: var(--bg-raised);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    box-shadow: 0 16px 40px rgb(0 0 0 / 0.45);
    color: var(--text-1);
    transform-origin: bottom left;
    animation: pop-in 180ms cubic-bezier(0.2, 0.9, 0.3, 1.2);
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 10px;
  }
  .title {
    font-size: calc(13px * var(--font-scale, 1));
    font-weight: 600;
  }
  .badge {
    padding: 2px 8px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--warn) 16%, transparent);
    color: var(--warn);
    font-size: calc(10.5px * var(--font-scale, 1));
    font-weight: 600;
  }
  .badge.focus {
    background: color-mix(in srgb, var(--accent) 16%, transparent);
    color: var(--accent);
  }
  .ask {
    margin: 0 0 10px;
    color: var(--text-2);
    font-size: calc(12px * var(--font-scale, 1));
  }
  .presets {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 6px;
  }
  .chip {
    height: 32px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-sidebar);
    color: var(--text-2);
    font: inherit;
    font-size: calc(12.5px * var(--font-scale, 1));
    cursor: pointer;
    transition: border-color 120ms, color 120ms, background 120ms;
  }
  .chip:hover {
    color: var(--text-1);
    border-color: var(--border-strong);
  }
  .chip.sel {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    color: var(--accent);
  }
  .custom {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-top: 10px;
    color: var(--text-2);
    font-size: calc(12px * var(--font-scale, 1));
  }
  .custom input {
    width: 72px;
    height: 28px;
    padding: 0 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-sidebar);
    color: var(--text-1);
    font-family: var(--font-mono);
    font-size: calc(12px * var(--font-scale, 1));
    outline: none;
  }
  .custom input:focus {
    border-color: var(--accent);
    box-shadow: var(--focus-ring);
  }
  .preview {
    margin: 14px 0 4px;
    color: var(--text-1);
    font-family: var(--font-mono);
    font-size: calc(30px * var(--font-scale, 1));
    font-variant-numeric: tabular-nums;
    text-align: center;
    letter-spacing: 0.02em;
  }

  .stage {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 6px 0 4px;
  }
  .glow {
    position: absolute;
    top: 0;
    left: 50%;
    width: 150px;
    height: 150px;
    margin-left: -75px;
    border-radius: 50%;
    background: radial-gradient(circle, color-mix(in srgb, var(--accent) 28%, transparent), transparent 65%);
    opacity: 0.35;
    pointer-events: none;
  }
  .stage.running .glow {
    animation: breathe 3s ease-in-out infinite;
  }
  .stage.done .glow {
    background: radial-gradient(circle, color-mix(in srgb, var(--ok) 34%, transparent), transparent 65%);
    opacity: 0.8;
  }
  .glass {
    position: relative;
    animation: flip 900ms cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  .frame {
    fill: none;
    stroke: var(--text-2);
    stroke-width: 2;
    stroke-linecap: round;
  }
  .plate {
    fill: var(--text-2);
  }
  .sand {
    fill: var(--accent);
  }
  .stage.done .sand {
    fill: var(--ok);
  }
  .stream {
    stroke: var(--accent);
    stroke-width: 1.6;
    stroke-dasharray: 2 3;
    animation: fall 0.5s linear infinite;
  }
  .clock {
    position: relative;
    margin-top: 10px;
    font-family: var(--font-mono);
    font-size: calc(32px * var(--font-scale, 1));
    font-weight: 500;
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.02em;
    animation: rise 600ms 200ms both cubic-bezier(0.2, 0.8, 0.2, 1);
  }
  .stage.done .clock {
    color: var(--ok);
  }
  .bar {
    position: relative;
    width: 100%;
    height: 3px;
    margin-top: 10px;
    border-radius: 2px;
    background: var(--border);
    overflow: hidden;
  }
  .bar span {
    display: block;
    height: 100%;
    background: var(--accent);
    transition: width 250ms linear;
  }
  .stage.done .bar span {
    background: var(--ok);
  }

  .actions {
    display: flex;
    gap: 6px;
    margin-top: 14px;
  }
  .btn {
    flex: 1;
    height: 32px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    background: none;
    color: var(--text-1);
    font: inherit;
    font-size: calc(12.5px * var(--font-scale, 1));
    cursor: pointer;
  }
  .btn:hover:not(:disabled) {
    background: color-mix(in srgb, var(--text-1) 8%, transparent);
  }
  .btn:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .btn.primary {
    border-color: var(--accent);
    background: var(--accent);
    color: #fff;
  }
  .btn.primary:hover {
    background: color-mix(in srgb, var(--accent) 86%, #000);
  }

  @keyframes pop-in {
    from { opacity: 0; transform: translateY(6px) scale(0.96); }
  }
  @keyframes flip {
    0% { opacity: 0; transform: rotate(180deg) scale(0.6); }
    60% { opacity: 1; }
    100% { transform: rotate(0) scale(1); }
  }
  @keyframes rise {
    from { opacity: 0; transform: translateY(8px); filter: blur(6px); }
  }
  @keyframes breathe {
    0%, 100% { opacity: 0.25; transform: scale(0.92); }
    50% { opacity: 0.6; transform: scale(1.05); }
  }
  @keyframes fall {
    to { stroke-dashoffset: -5; }
  }
  @keyframes pulse {
    50% { opacity: 0.3; }
  }
  @keyframes tip {
    0%, 80% { transform: rotate(0); }
    90%, 100% { transform: rotate(180deg); }
  }
  @media (prefers-reduced-motion: reduce) {
    .pop, .glass, .clock, .stream, .glow, .trigger svg {
      animation: none !important;
    }
  }
</style>
