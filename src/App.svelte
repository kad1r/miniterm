<script lang="ts">
  import { onMount, untrack } from "svelte"
  import { app, bootstrap, commit, dismissToast, notify } from "./store/app.svelte"
  import { findNode, updateWorkspace } from "./store/tree"
  import {
    activate, closePane, listenExits, rememberFocus, restart, sessions,
  } from "./store/sessions.svelte"
  import { fontAction } from "./store/font"
  import { applyFontAction, initFontScale } from "./store/font.svelte"
  import { initLocale, t } from "./i18n/locale.svelte"
  import Sidebar from "./lib/Sidebar.svelte"
  import Settings from "./lib/Settings.svelte"
  import Wizard from "./lib/Wizard.svelte"
  import LayoutDialog from "./lib/LayoutDialog.svelte"
  import TerminalGrid from "./lib/TerminalGrid.svelte"

  let wizardOpen = $state(false)
  // Held as an id, not as the node: the workspace is re-created on every commit,
  // so a captured object would go stale the moment the dialog applies its patch.
  let addTerminalTo = $state<string | null>(null)

  onMount(async () => {
    initLocale()
    initFontScale()
    const exits = listenExits()
    await bootstrap()
    await exits
  })

  /** Ctrl/Cmd +, − and 0. Handled at the window rather than per-component so it
   *  works with focus anywhere, including inside a terminal — TerminalPane hands
   *  these keys back to the window instead of forwarding them to the shell. */
  function onKeydown(e: KeyboardEvent) {
    const action = fontAction(e)
    if (!action) return
    e.preventDefault()
    applyFontAction(action)
  }

  /** Ctrl/Cmd + wheel, the mouse spelling of the same zoom.
   *
   *  Capture phase and stopPropagation because xterm's own wheel listener would
   *  otherwise scroll the scrollback at the same time, and preventDefault with
   *  an explicitly non-passive listener because the WebView's default action for
   *  Ctrl+wheel is to zoom the entire app chrome. */
  onMount(() => {
    const onWheel = (e: WheelEvent) => {
      if ((!e.ctrlKey && !e.metaKey) || e.deltaY === 0) return
      e.preventDefault()
      e.stopPropagation()
      applyFontAction(e.deltaY < 0 ? "in" : "out")
    }
    window.addEventListener("wheel", onWheel, { capture: true, passive: false })
    return () => window.removeEventListener("wheel", onWheel, { capture: true })
  })

  // Seçili workspace değişince oturumları hazırla.
  $effect(() => {
    const id = app.activeWorkspaceId
    if (id) untrack(() => void activate(id))
  })

  function workspaceById(id: string) {
    const node = findNode(app.config.tree, id)
    return node && node.kind === "workspace" ? node : null
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="shell">
  {#if !app.ready}
    <div class="boot">{t("app.loading")}</div>
  {:else}
    <Sidebar onnew={() => (wizardOpen = true)} onaddterminal={(id) => (addTerminalTo = id)} />
    <main>
      {#if app.view === "settings"}
        <Settings />
      {:else if app.activeWorkspaceId === null}
        <div class="placeholder">{t("app.placeholder")}</div>
      {:else}
        {#each [...sessions.live].sort() as wsId (wsId)}
          {@const ws = workspaceById(wsId)}
          {#if ws}
            {@const active = wsId === app.activeWorkspaceId}
            {@const slot = sessions.byWorkspace[wsId]}
            <div class="layer" class:hidden={!active}>
              <TerminalGrid
                workspace={ws}
                {active}
                sessionIds={active ? (slot?.ids ?? []) : []}
                exitCodes={active ? (slot?.exits ?? []) : []}
                focusedIndex={slot?.focused ?? 0}
                onrestart={(i) => void restart(wsId, i)}
                onfocuspane={(i) => rememberFocus(wsId, i)}
                onclose={(i) => {
                  void closePane(wsId, i)
                  notify(t("grid.paneClosed"))
                }}
                onsizes={(patch) =>
                  commit((c) => ({ ...c, tree: updateWorkspace(c.tree, wsId, patch) }))}
              />
            </div>
          {/if}
        {/each}
      {/if}
    </main>
    {#if wizardOpen}
      <Wizard onclose={() => (wizardOpen = false)} />
    {/if}
    {#if addTerminalTo}
      {@const target = workspaceById(addTerminalTo)}
      {#if target}
        <LayoutDialog workspace={target} onclose={() => (addTerminalTo = null)} />
      {/if}
    {/if}
  {/if}

  {#if app.toast}
    <button class="toast" class:err={app.toast.tone === "error"} onclick={dismissToast}>
      {app.toast.text}
    </button>
  {/if}
</div>

<style>
  .shell {
    display: flex;
    width: 100%;
    height: 100%;
    overflow: hidden;
  }
  main {
    flex: 1;
    min-width: 0;
    position: relative;
    background: var(--bg);
  }
  /* Breathing room around the grid. It sits here rather than on `main` because
     the layers are absolutely positioned, and `inset: 0` resolves against the
     padding box — padding on the parent would not inset them at all. */
  .layer {
    position: absolute;
    inset: 0;
    padding: 8px;
  }
  .layer.hidden {
    visibility: hidden;
    pointer-events: none;
  }
  .boot,
  .placeholder {
    display: grid;
    place-items: center;
    width: 100%;
    height: 100%;
    color: var(--text-dim);
    font-size: calc(13px * var(--font-scale, 1));
  }
  .toast {
    position: fixed;
    right: 16px;
    bottom: 16px;
    padding: 9px 14px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-raised);
    color: var(--text);
    font: inherit;
    font-size: calc(13px * var(--font-scale, 1));
    cursor: pointer;
  }
  .toast.err {
    border-color: var(--err);
    color: var(--err);
  }
</style>
