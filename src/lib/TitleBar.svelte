<script lang="ts">
  import { onMount } from "svelte"
  import { closeWindow, minimizeWindow, onMaximizedChange, toggleMaximizeWindow } from "../ipc"
  import { t } from "../i18n/locale.svelte"

  /** The window has no system frame; this strip is its caption. Dragging,
   *  double-click maximise and drag-to-snap come from `data-tauri-drag-region`,
   *  which hands the gesture to the OS — so they behave exactly as on a native
   *  title bar. Only the three buttons are ours. */
  let { title = "", board = null, onboard }: {
    title?: string
    /** Whether the task board is showing; null hides the toggle (no workspace). */
    board?: boolean | null
    onboard?: () => void
  } = $props()

  let maximized = $state(false)

  onMount(() => {
    let stop = () => {}
    let gone = false
    onMaximizedChange((m) => (maximized = m))
      .then((unlisten) => (gone ? unlisten() : (stop = unlisten)))
      .catch(() => {})
    return () => {
      gone = true
      stop()
    }
  })
</script>

<header class="titlebar" class:maximized data-tauri-drag-region>
  <span class="name" data-tauri-drag-region>MINITERM</span>
  <span class="title" data-tauri-drag-region>{title}</span>
  <div class="controls">
    {#if board !== null}
      <button
        class="tool"
        class:on={board}
        onclick={() => onboard?.()}
        title={board ? t("window.terminals") : t("window.board")}
        aria-label={t("window.board")}
        aria-pressed={board}
      >
        <svg class="glyph" viewBox="0 0 24 24" aria-hidden="true"><rect x="3" y="4" width="5" height="16" rx="1.2"></rect><rect x="9.5" y="4" width="5" height="10" rx="1.2"></rect><rect x="16" y="4" width="5" height="13" rx="1.2"></rect></svg>
      </button>
    {/if}
    <button onclick={() => void minimizeWindow()} title={t("window.minimize")} aria-label={t("window.minimize")}>
      <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M0 5h10" /></svg>
    </button>
    <button
      onclick={() => void toggleMaximizeWindow()}
      title={maximized ? t("window.restore") : t("window.maximize")}
      aria-label={maximized ? t("window.restore") : t("window.maximize")}
    >
      {#if maximized}
        <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M2.5 2.5v-2h7v7h-2M.5 2.5h7v7h-7z" /></svg>
      {:else}
        <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M.5.5h9v9h-9z" /></svg>
      {/if}
    </button>
    <button class="close" onclick={() => void closeWindow()} title={t("window.close")} aria-label={t("window.close")}>
      <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M0 0l10 10M10 0L0 10" /></svg>
    </button>
  </div>
</header>

<style>
  .titlebar {
    flex: 0 0 auto;
    position: relative;
    display: flex;
    align-items: center;
    height: 34px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-sidebar);
    color: var(--text-3);
    font-size: 12px;
    user-select: none;
  }
  .name {
    padding: 0 14px;
    align-self: stretch;
    display: flex;
    align-items: center;
    font-family: var(--font-mono);
    font-size: 11.5px;
    font-weight: 500;
    letter-spacing: 0.08em;
  }
  /* Centred on the window, not on the space left between the name and the
     buttons, so it does not shift when either side changes width. */
  .title {
    position: absolute;
    left: 50%;
    max-width: 50%;
    transform: translateX(-50%);
    overflow: hidden;
    color: var(--text-2);
    text-overflow: ellipsis;
    white-space: nowrap;
    /* The strip underneath is the drag region; a click on the label must reach
       it instead of being swallowed here. */
    pointer-events: none;
  }
  .controls {
    display: flex;
    align-self: stretch;
    margin-left: auto;
  }
  button {
    width: 46px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--text-2);
    cursor: default;
  }
  button:hover {
    background: color-mix(in srgb, var(--text-1) 10%, transparent);
    color: var(--text-1);
  }
  button:active {
    background: color-mix(in srgb, var(--text-1) 16%, transparent);
  }
  button.close:hover {
    background: #c42b1c;
    color: #fff;
  }
  button.close:active {
    background: #a52416;
  }
  button:focus-visible {
    outline: none;
    box-shadow: inset 0 0 0 1.5px var(--accent);
  }
  svg {
    width: 10px;
    height: 10px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1;
    shape-rendering: crispEdges;
  }
  /* App actions sit left of the window buttons, set apart by a hairline. */
  .tool {
    width: 40px;
    margin-right: 6px;
    border-right: 1px solid var(--border);
  }
  .tool.on {
    color: var(--accent);
  }
  svg.glyph {
    width: 15px;
    height: 15px;
    stroke-width: 1.6;
    shape-rendering: geometricPrecision;
  }
  /* The diagonals of the cross look ragged without antialiasing. */
  .close svg {
    shape-rendering: geometricPrecision;
  }
</style>
