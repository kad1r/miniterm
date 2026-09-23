<script lang="ts">
  import { onMount, untrack } from "svelte"
  import { app, bootstrap, commit, dismissToast, notify } from "./store/app.svelte"
  import { findNode, updateWorkspace } from "./store/tree"
  import {
    activate, closePane, listenExits, minimizePane, rememberFocus, restart, restorePane,
    sessions, toggleMaximize,
  } from "./store/sessions.svelte"
  import { MAX_TERMINALS } from "./store/layout"
  import { fontAction, termFontSize } from "./store/font"
  import { applyFontAction, fontScale, initFontScale } from "./store/font.svelte"
  import { initTheme } from "./store/theme.svelte"
  import { TERM_FONT_SIZE } from "./term/theme"
  import { initLocale, locale, t } from "./i18n/locale.svelte"
  import {
    isHelpChord, isNewTerminalChord, isNewWorkspaceChord, isSettingsChord,
  } from "./term/shortcuts"
  import Sidebar from "./lib/Sidebar.svelte"
  import Settings from "./lib/Settings.svelte"
  import Wizard from "./lib/Wizard.svelte"
  import LayoutDialog from "./lib/LayoutDialog.svelte"
  import Shortcuts from "./lib/Shortcuts.svelte"
  import TerminalGrid from "./lib/TerminalGrid.svelte"
  import UpdateBanner from "./lib/UpdateBanner.svelte"
  import { runCheck } from "./store/update.svelte"

  let wizardOpen = $state(false)
  let helpOpen = $state(false)
  // Held as an id, not as the node: the workspace is re-created on every commit,
  // so a captured object would go stale the moment the dialog applies its patch.
  let layoutDialog = $state<{ id: string; mode: "add" | "layout" } | null>(null)

  onMount(async () => {
    initLocale()
    initFontScale()
    initTheme()
    const exits = listenExits()
    await bootstrap()
    void runCheck(true)
    await exits
  })

  /** Ctrl/Cmd +, − and 0. Handled at the window rather than per-component so it
   *  works with focus anywhere, including inside a terminal — TerminalPane hands
   *  these keys back to the window instead of forwarding them to the shell. */
  function onKeydown(e: KeyboardEvent) {
    const action = fontAction(e)
    if (action) {
      e.preventDefault()
      applyFontAction(action)
      return
    }
    if (isHelpChord(e)) {
      e.preventDefault()
      helpOpen = !helpOpen
      return
    }
    if (isSettingsChord(e)) {
      e.preventDefault()
      app.view = app.view === "settings" ? "workspace" : "settings"
      return
    }
    if (isNewWorkspaceChord(e)) {
      e.preventDefault()
      wizardOpen = true
      return
    }
    // Adding a terminal needs a workspace to add it to; with none selected the
    // chord is a no-op and the keystroke is left alone.
    if (isNewTerminalChord(e) && app.activeWorkspaceId) {
      e.preventDefault()
      layoutDialog = { id: app.activeWorkspaceId, mode: "add" }
    }
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

  // Names the current context in the header breadcrumb. Null on the settings
  // view or when nothing is selected, so the crumb simply disappears.
  const activeWs = $derived(
    app.activeWorkspaceId ? workspaceById(app.activeWorkspaceId) : null,
  )

  // Bottom status bar figures for the active workspace: which shell it runs, how
  // many panes hold a session and how many of those are still alive.
  const activeSlot = $derived(
    app.activeWorkspaceId ? sessions.byWorkspace[app.activeWorkspaceId] : null,
  )
  const sessionCount = $derived(
    activeSlot ? activeSlot.ids.filter((x) => x !== null).length : 0,
  )
  const runningCount = $derived(
    activeSlot
      ? activeSlot.ids.filter((x, i) => x !== null && activeSlot.exits[i] === undefined).length
      : 0,
  )
  const shellName = $derived.by(() => {
    if (!activeWs) return ""
    const wanted = activeWs.shellId ?? app.config.defaultShellId
    return (app.shells.find((s) => s.id === wanted) ?? app.shells[0])?.name ?? ""
  })
  const fontPx = $derived(termFontSize(TERM_FONT_SIZE, fontScale.level))
</script>

<svelte:window onkeydown={onKeydown} />

<div class="shell">
  {#if !app.ready}
    <div class="boot">{t("app.loading")}</div>
  {:else}
    <UpdateBanner />
    <div class="body">
    <Sidebar
      onnew={() => (wizardOpen = true)}
      onaddterminal={(id) => (layoutDialog = { id, mode: "add" })}
      onchangelayout={(id) => (layoutDialog = { id, mode: "layout" })}
    />
    <main>
      <div class="stage">
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
                minimized={slot?.minimized ?? []}
                maximized={slot?.maximized ?? null}
                onrestart={(i) => void restart(wsId, i)}
                onfocuspane={(i) => rememberFocus(wsId, i)}
                onclose={(i) => {
                  void closePane(wsId, i)
                  notify(t("grid.paneClosed"))
                }}
                onminimize={(i) => {
                  minimizePane(wsId, i)
                  notify(t("grid.paneMinimized"))
                }}
                onmaximize={(i) => toggleMaximize(wsId, i)}
                onrestore={(i) => {
                  if (restorePane(wsId, i)) notify(t("grid.paneRestored"))
                  else notify(t("grid.gridFull", { max: MAX_TERMINALS }), "error")
                }}
                onsizes={(patch) =>
                  commit((c) => ({ ...c, tree: updateWorkspace(c.tree, wsId, patch) }))}
              />
            </div>
          {/if}
        {/each}
      {/if}
      </div>
      <footer class="statusbar">
        {#if activeWs}
          <span class="sb-strong">{shellName}</span>
          <span>{t("statusbar.sessions", { n: sessionCount })} · {t("statusbar.running", { n: runningCount })}</span>
        {/if}
        <div class="sb-spacer"></div>
        <span>{fontPx} px</span>
        <span class="sb-strong">{locale.current.toUpperCase()}</span>
      </footer>
    </main>
    </div>
    {#if wizardOpen}
      <Wizard onclose={() => (wizardOpen = false)} />
    {/if}
    {#if helpOpen}
      <Shortcuts onclose={() => (helpOpen = false)} />
    {/if}
    {#if layoutDialog}
      {@const target = workspaceById(layoutDialog.id)}
      {#if target}
        <LayoutDialog
          workspace={target}
          mode={layoutDialog.mode}
          onclose={() => (layoutDialog = null)}
        />
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
    flex-direction: column;
    width: 100%;
    height: 100%;
    overflow: hidden;
    background: var(--bg-app);
  }
  .body {
    flex: 1 1 auto;
    display: flex;
    min-height: 0;
    overflow: hidden;
  }
  main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg-app);
  }
  /* Holds the workspace layers (or settings). Positioned so the absolutely
     positioned layers inset against it, leaving the status bar its own row. */
  .stage {
    position: relative;
    flex: 1;
    min-height: 0;
  }
  .layer {
    position: absolute;
    inset: 0;
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
  /* Thin bottom strip: shell + session tally on the left, font size and locale
     on the right, all in the mono face like a real terminal status line. */
  .statusbar {
    flex: 0 0 auto;
    display: flex;
    align-items: center;
    gap: 16px;
    height: 30px;
    padding: 0 14px;
    border-top: 1px solid var(--border);
    background: var(--bg-app);
    color: var(--text-3);
    font-family: var(--font-mono);
    font-size: calc(10.5px * var(--font-scale, 1));
  }
  .sb-strong {
    color: var(--text-2);
  }
  .sb-spacer {
    flex: 1 1 auto;
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
