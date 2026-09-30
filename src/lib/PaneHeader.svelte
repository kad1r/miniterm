<script lang="ts">
  import { t } from "../i18n/locale.svelte"

  /** The bar above one terminal: status dot, agent chip (double-click to
   *  rename), the workspace location, the @ introduction button and — when the
   *  grid has more than one pane — minimize, maximize and close. */
  let {
    index, name, folderName, path, dead, closable, maximized, canIntroduce,
    onrename, onrenamedone, onintroduce, onminimize, onmaximize, onclose,
  }: {
    index: number
    name: string
    folderName: string
    path: string
    dead: boolean
    closable: boolean
    maximized: boolean
    canIntroduce: boolean
    onrename: (draft: string) => void
    /** Rename finished either way; the grid hands focus back to the shell. */
    onrenamedone: () => void
    onintroduce: () => void
    onminimize: () => void
    onmaximize: () => void
    onclose: () => void
  } = $props()

  let editing = $state(false)
  let draft = $state("")

  function startRename() {
    draft = name
    editing = true
  }

  function finishRename(save: boolean) {
    if (!editing) return
    editing = false
    if (save) onrename(draft)
    onrenamedone()
  }

  function autofocus(el: HTMLInputElement) {
    el.focus()
    el.select()
  }

  /** Keys typed while renaming belong to the box alone. Bound natively on the
   *  input: Svelte delegates `onkeydown` to the app root, so a stopPropagation
   *  there runs only after the grid's own keydown listener has already seen the
   *  event — and Ctrl+Shift+W would close the pane mid-rename. */
  function renameKeys(el: HTMLInputElement) {
    const onKey = (e: KeyboardEvent) => {
      e.stopPropagation()
      if (e.key === "Enter") finishRename(true)
      else if (e.key === "Escape") finishRename(false)
    }
    el.addEventListener("keydown", onKey)
    return { destroy: () => el.removeEventListener("keydown", onKey) }
  }
</script>

<div class="pane-header">
  <span class="pane-dot" class:dead></span>
  {#if editing}
    <input
      class="agent-input"
      aria-label={t("agents.renameLabel")}
      maxlength="24"
      bind:value={draft}
      use:autofocus
      use:renameKeys
      onblur={() => finishRename(true)}
    />
  {:else}
    <button
      class="agent"
      type="button"
      title={t("agents.renameHint", { name })}
      ondblclick={startRename}
    >
      {name}
    </button>
  {/if}
  <div class="pane-loc" title={path}>
    <span class="pane-name">{folderName}</span>
    {#if path}<span class="pane-path">{path}</span>{/if}
  </div>
  <button
    class="pane-btn intro"
    type="button"
    title={t("agents.introduce", { name })}
    aria-label={t("agents.introduce", { name })}
    disabled={!canIntroduce}
    onclick={onintroduce}
  >
    <svg viewBox="0 0 12 12" aria-hidden="true">
      <circle cx="6" cy="6" r="2" />
      <path d="M8 6v.8a1.4 1.4 0 0 0 2.8 0V6A4.8 4.8 0 1 0 8.4 10" />
    </svg>
  </button>
  {#if closable}
    <div class="pane-actions">
      <button
        class="pane-btn"
        type="button"
        title={t("grid.minimizePane", { n: index + 1 })}
        aria-label={t("grid.minimizePane", { n: index + 1 })}
        onclick={onminimize}
      >
        <svg viewBox="0 0 12 12" aria-hidden="true">
          <path d="M3 8h6" />
        </svg>
      </button>
      <button
        class="pane-btn"
        type="button"
        title={maximized ? t("grid.unmaximizePane") : t("grid.maximizePane", { n: index + 1 })}
        aria-label={maximized ? t("grid.unmaximizePane") : t("grid.maximizePane", { n: index + 1 })}
        aria-pressed={maximized}
        onclick={onmaximize}
      >
        {#if maximized}
          <svg viewBox="0 0 12 12" aria-hidden="true">
            <path d="M5 5h4v4H5zM3 7V3h4" />
          </svg>
        {:else}
          <svg viewBox="0 0 12 12" aria-hidden="true">
            <path d="M3 3h6v6H3z" />
          </svg>
        {/if}
      </button>
      <button
        class="pane-btn danger"
        type="button"
        title={t("grid.closePane", { n: index + 1 })}
        aria-label={t("grid.closePane", { n: index + 1 })}
        onclick={onclose}
      >
        <svg viewBox="0 0 12 12" aria-hidden="true">
          <path d="M3 3l6 6M9 3l-6 6" />
        </svg>
      </button>
    </div>
  {/if}
</div>

<style>
  /* A real header bar above the shell, not an overlay — the buttons can never
     sit on top of terminal text now, so typing no longer collides with them. */
  .pane-header {
    flex: 0 0 auto;
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 32px;
    padding: 0 6px 0 10px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-elevated);
  }
  .pane-dot {
    flex: 0 0 auto;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--ok);
  }
  .pane-dot.dead {
    background: var(--err);
  }
  .agent {
    flex: 0 0 auto;
    max-width: 40%;
    padding: 1px 8px;
    overflow: hidden;
    border: 1px solid color-mix(in srgb, var(--accent) 35%, transparent);
    border-radius: 999px;
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    color: var(--accent);
    font: inherit;
    font-size: calc(11px * var(--font-scale, 1));
    font-weight: 600;
    white-space: nowrap;
    text-overflow: ellipsis;
    cursor: text;
  }
  .agent-input {
    flex: 0 0 auto;
    width: 120px;
    padding: 1px 8px;
    border: 1px solid var(--accent);
    border-radius: 999px;
    background: var(--bg-elevated);
    color: var(--text-1);
    font: inherit;
    font-size: calc(11px * var(--font-scale, 1));
    font-weight: 600;
    outline: none;
    box-shadow: var(--focus-ring);
  }
  .pane-loc {
    flex: 1;
    display: flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
    overflow: hidden;
  }
  .pane-name {
    flex: 0 0 auto;
    color: var(--text-1);
    font-size: calc(11.5px * var(--font-scale, 1));
    font-weight: 600;
  }
  .pane-path {
    min-width: 0;
    overflow: hidden;
    color: var(--text-3);
    font-family: var(--font-mono);
    font-size: calc(10.5px * var(--font-scale, 1));
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .pane-actions {
    flex: 0 0 auto;
    display: flex;
    gap: 2px;
  }
  .pane-btn {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--text-2);
    cursor: pointer;
  }
  .pane-btn:hover,
  .pane-btn:focus-visible {
    background: color-mix(in srgb, var(--text-1) 10%, transparent);
    color: var(--text-1);
    outline: none;
  }
  .pane-btn:disabled {
    opacity: 0.35;
    cursor: default;
    background: none;
  }
  .pane-btn.danger:hover,
  .pane-btn.danger:focus-visible {
    background: var(--err);
    color: #fff;
  }
  .pane-btn[aria-pressed="true"] {
    color: var(--accent);
  }
  .pane-btn svg {
    width: 12px;
    height: 12px;
    stroke: currentColor;
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-linejoin: round;
    fill: none;
  }
</style>
