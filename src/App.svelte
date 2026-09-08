<script lang="ts">
  import { onMount, untrack } from "svelte"
  import { app, bootstrap, commit, dismissToast } from "./store/app.svelte"
  import { findNode, updateWorkspace } from "./store/tree"
  import { activate, listenExits, restart, sessions } from "./store/sessions.svelte"
  import Sidebar from "./lib/Sidebar.svelte"
  import Settings from "./lib/Settings.svelte"
  import Wizard from "./lib/Wizard.svelte"
  import TerminalGrid from "./lib/TerminalGrid.svelte"

  let wizardOpen = $state(false)

  onMount(async () => {
    const exits = listenExits()
    await bootstrap()
    await exits
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

<div class="shell">
  {#if !app.ready}
    <div class="boot">Yükleniyor…</div>
  {:else}
    <Sidebar onnew={() => (wizardOpen = true)} />
    <main>
      {#if app.view === "settings"}
        <Settings />
      {:else if app.activeWorkspaceId === null}
        <div class="placeholder">Bir workspace seç ya da yeni bir tane oluştur.</div>
      {:else}
        {#each [...sessions.live].sort() as wsId (wsId)}
          {@const ws = workspaceById(wsId)}
          {#if ws}
            {@const active = wsId === app.activeWorkspaceId}
            {@const slot = sessions.byWorkspace[wsId]}
            <div class="layer" class:hidden={!active}>
              <TerminalGrid
                workspace={ws}
                sessionIds={active ? (slot?.ids ?? []) : []}
                exitCodes={active ? (slot?.exits ?? []) : []}
                onrestart={(i) => void restart(wsId, i)}
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
    width: 100vw;
    height: 100vh;
    overflow: hidden;
  }
  main {
    flex: 1;
    min-width: 0;
    position: relative;
    background: var(--bg);
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
    font-size: 13px;
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
    font-size: 13px;
    cursor: pointer;
  }
  .toast.err {
    border-color: var(--err);
    color: var(--err);
  }
</style>
