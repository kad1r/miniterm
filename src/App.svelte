<script lang="ts">
  import { onMount } from "svelte";
  import { app, bootstrap, dismissToast } from "./store/app.svelte";

  onMount(bootstrap);
</script>

{#if !app.ready}
  <main class="center">yükleniyor…</main>
{:else}
  <main class="center">
    <p>{app.config.tree.length} kök düğüm · {app.shells.length} shell bulundu</p>
    <p class="dim">varsayılan shell: {app.config.defaultShellId || "yok"}</p>
  </main>
{/if}

{#if app.toast}
  <div class="toast" class:error={app.toast.tone === "error"}>
    <span>{app.toast.text}</span>
    <button onclick={dismissToast}>kapat</button>
  </div>
{/if}

<style>
  .center {
    display: grid;
    place-items: center;
    align-content: center;
    gap: 4px;
    height: 100%;
  }
  .dim { color: var(--text-dim); }
  .toast {
    position: fixed;
    right: 16px;
    bottom: 16px;
    display: flex;
    gap: 12px;
    align-items: center;
    max-width: 420px;
    padding: 10px 12px;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 6px;
  }
  .toast.error { border-color: var(--err); }
  button {
    background: none;
    border: none;
    color: var(--text-dim);
    cursor: pointer;
  }
</style>
