<script lang="ts">
  import { onMount } from "svelte"
  import { app, bootstrap, dismissToast, notify } from "./store/app.svelte"
  import Sidebar from "./lib/Sidebar.svelte"
  import TerminalPane from "./lib/TerminalPane.svelte"
  import { spawnSession } from "./ipc"

  onMount(bootstrap)

  let previewId = $state<number | null>(null)

  async function preview() {
    const shell = app.shells[0]
    if (!shell) return
    previewId = await spawnSession({
      cwd: ".",
      program: shell.program,
      args: shell.args,
      initialCommand: null,
      cols: 80,
      rows: 24,
    })
  }
</script>

<div class="shell">
  {#if !app.ready}
    <div class="boot">Yükleniyor…</div>
  {:else}
    <Sidebar onnew={() => notify("Sihirbaz henüz bağlanmadı")} />
    <main>
      {#if app.view === "settings"}
        <div class="placeholder">Ayarlar</div>
      {:else if app.activeWorkspaceId}
        {#if previewId === null}
          <div class="placeholder"><button onclick={preview}>Terminal başlat</button></div>
        {:else}
          <TerminalPane sessionId={previewId} />
        {/if}
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
