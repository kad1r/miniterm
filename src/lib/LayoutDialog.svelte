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

  let { workspace, onclose }: { workspace: Workspace; onclose: () => void } = $props()

  const current = $derived(terminalCount(workspace))
  const counts = $derived(addableCounts(workspace))
  const atLimit = $derived(current >= MAX_TERMINALS)

  // Opened from "add a terminal", so it starts on one more than today — the user
  // can still walk the picker up to the limit before confirming. untrack because
  // these two are the user's working copy: once the dialog is up, a change to the
  // workspace must not yank the selection out from under them.
  const initial = untrack(() => Math.min(terminalCount(workspace) + 1, MAX_TERMINALS))
  let count = $state(initial)
  let layout = $state<GridLayout>(untrack(() => defaultLayoutFor(workspace, initial)))
  let dialogEl = $state<HTMLDialogElement | undefined>(undefined)

  $effect(() => {
    if (dialogEl) dialogEl.showModal()
  })

  function setCount(n: number) {
    count = n
    layout = defaultLayoutFor(workspace, n)
  }

  function apply() {
    if (count === current) {
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
    notify(t("layout.added", { name: workspace.name, count: count - current }))
    onclose()
  }
</script>

<dialog
  bind:this={dialogEl}
  aria-modal="true"
  aria-label={t("layout.title", { name: workspace.name })}
  oncancel={(e) => { e.preventDefault(); onclose() }}
>
  <div class="dialog-inner" role="presentation">
    <header>
      <h2>{t("layout.title", { name: workspace.name })}</h2>
    </header>

    <div class="body">
      {#if atLimit}
        <p class="note">{t("layout.atLimit", { max: MAX_TERMINALS })}</p>
      {:else}
        <LayoutPicker {count} {layout} {counts} oncount={setCount} onlayout={(l) => (layout = l)} />
        <p class="note">{t("layout.hint")}</p>
      {/if}
    </div>

    <footer>
      <button class="ghost" onclick={onclose}>{t("settings.cancel")}</button>
      <span class="spacer"></span>
      <button class="primary" disabled={atLimit || count === current} onclick={apply}>
        {t("layout.apply")}
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
