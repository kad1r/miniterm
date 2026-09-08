<script lang="ts">
  import { onMount } from "svelte"
  import { app, bootstrap, dismissToast, notify } from "./store/app.svelte"
  import Sidebar from "./lib/Sidebar.svelte"
  import TerminalGrid from "./lib/TerminalGrid.svelte"
  import { findNode, updateWorkspace } from "./store/tree"
  import { commit } from "./store/app.svelte"

  onMount(bootstrap)

  const activeWorkspace = $derived.by(() => {
    if (!app.activeWorkspaceId) return null
    const node = findNode(app.config.tree, app.activeWorkspaceId)
    return node && node.kind === "workspace" ? node : null
  })
</script>

<div class="shell">
  {#if !app.ready}
    <div class="boot">Yükleniyor…</div>
  {:else}
    <Sidebar onnew={() => notify("Sihirbaz henüz bağlanmadı")} />
    <main>
      {#if app.view === "settings"}
        <div class="placeholder">Ayarlar</div>
      {:else if activeWorkspace}
        {@const ws = activeWorkspace}
        <TerminalGrid
          workspace={ws}
          sessionIds={Array(ws.rows * ws.cols).fill(null)}
          exitCodes={Array(ws.rows * ws.cols).fill(undefined)}
          onrestart={() => {}}
          onsizes={(patch) =>
            commit((c) => ({ ...c, tree: updateWorkspace(c.tree, ws.id, patch) }))}
        />
      {:else}
        <div class="placeholder">Bir workspace seç ya da yeni bir tane oluştur.</div>
      {/if}
    </main>
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
    background: var(--bg);
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
