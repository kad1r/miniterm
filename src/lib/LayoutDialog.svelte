<script lang="ts">
  import { untrack } from "svelte"
  import { app, commit, notify } from "../store/app.svelte"
  import { updateWorkspace } from "../store/tree"
  import { activate } from "../store/sessions.svelte"
  import { addableCounts, defaultLayoutFor, relayout, terminalCount } from "../store/relayout"
  import { MAX_TERMINALS, type GridLayout } from "../store/layout"
  import type { Workspace } from "../store/types"
  import LayoutPicker from "./LayoutPicker.svelte"
  import { t } from "../i18n/locale.svelte"

  /** `add` opens on one terminal more than today, `layout` opens on today's count
   *  so the picker is a pure reshape. Both modes offer the same controls once
   *  open — the mode only decides where the dialog starts and what it says. */
  let { workspace, mode = "add", onclose }: {
    workspace: Workspace
    mode?: "add" | "layout"
    onclose: () => void
  } = $props()

  const current = $derived(terminalCount(workspace))
  const counts = $derived(addableCounts(workspace))
  // A full workspace can still be reshaped (6 terminals is 1×6, 2×3, 3×2 or 6×1),
  // so the limit only closes the door on the mode that wanted to add one.
  const atLimit = $derived(mode === "add" && current >= MAX_TERMINALS)

  // untrack because these two are the user's working copy: once the dialog is up,
  // a change to the workspace must not yank the selection out from under them.
  const initial = untrack(() =>
    mode === "add" ? Math.min(terminalCount(workspace) + 1, MAX_TERMINALS) : terminalCount(workspace),
  )
  let count = $state(initial)
  let layout = $state<GridLayout>(untrack(() => defaultLayoutFor(workspace, initial)))
  let dialogEl = $state<HTMLDialogElement | undefined>(undefined)

  const title = $derived(
    t(mode === "add" ? "layout.title" : "layout.changeTitle", { name: workspace.name }),
  )

  $effect(() => {
    if (dialogEl) dialogEl.showModal()
  })

  function setCount(n: number) {
    count = n
    layout = defaultLayoutFor(workspace, n)
  }

  const changed = $derived(
    count !== current || layout.rows !== workspace.rows || layout.cols !== workspace.cols,
  )

  function apply() {
    if (!changed) {
      onclose()
      return
    }
    const patch = relayout(workspace, layout)
    commit((c) => ({ ...c, tree: updateWorkspace(c.tree, workspace.id, patch) }))
    app.activeWorkspaceId = workspace.id
    app.view = "workspace"
    // activate() reconciles the grid against the running sessions, but App only
    // calls it when the selected workspace changes — a relayout of the workspace
    // already on screen has to ask for it here.
    void activate(workspace.id)
    notify(
      count > current
        ? t("layout.added", { name: workspace.name, count: count - current })
        : t("layout.changed", { name: workspace.name, rows: layout.rows, cols: layout.cols }),
    )
    onclose()
  }
</script>

<dialog
  bind:this={dialogEl}
  aria-modal="true"
  aria-label={title}
  oncancel={(e) => { e.preventDefault(); onclose() }}
>
  <div class="dialog-inner" role="presentation">
    <header>
      <h2>{title}</h2>
    </header>

    <div class="body">
      {#if atLimit}
        <p class="note">{t("layout.atLimit", { max: MAX_TERMINALS })}</p>
      {:else}
        <LayoutPicker {count} {layout} {counts} oncount={setCount} onlayout={(l) => (layout = l)} />
        <p class="note">{t(mode === "add" ? "layout.hint" : "layout.changeHint")}</p>
      {/if}
    </div>

    <footer>
      <button class="ghost" onclick={onclose}>{t("settings.cancel")}</button>
      <span class="spacer"></span>
      <button class="primary" disabled={atLimit || !changed} onclick={apply}>
        {t(mode === "add" ? "layout.apply" : "layout.applyLayout")}
      </button>
    </footer>
  </div>
</dialog>

<style>
  dialog {
    padding: 0;
    border: none;
    border-radius: 10px;
    background: transparent;
    width: min(460px, 92vw);
    max-width: 92vw;
  }
  dialog::backdrop {
    background: rgb(0 0 0 / 0.5);
  }
  .dialog-inner {
    display: flex;
    flex-direction: column;
    width: 100%;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 10px;
    overflow: hidden;
  }
  header {
    display: flex;
    align-items: center;
    padding: 14px 18px;
    border-bottom: 1px solid var(--border);
  }
  h2 {
    flex: 1;
    margin: 0;
    font-size: calc(15px * var(--font-scale, 1));
    font-weight: 600;
  }
  .body {
    padding: 16px 18px;
  }
  .note {
    margin: 16px 0 0;
    color: var(--text-dim);
    font-size: calc(12px * var(--font-scale, 1));
  }
  footer {
    display: flex;
    gap: 8px;
    padding: 12px 18px;
    border-top: 1px solid var(--border);
  }
  .spacer {
    flex: 1;
  }
  .ghost,
  .primary {
    padding: 7px 14px;
    border-radius: 6px;
    font: inherit;
    font-size: calc(13px * var(--font-scale, 1));
    cursor: pointer;
  }
  .ghost {
    border: 1px solid var(--border);
    background: none;
    color: var(--text-dim);
  }
  .primary {
    border: 0;
    background: var(--accent);
    color: #fff;
  }
  .primary:disabled {
    opacity: 0.45;
    cursor: default;
  }
</style>
