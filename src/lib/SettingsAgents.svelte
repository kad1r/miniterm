<script lang="ts">
  import { onMount } from "svelte"
  import { notify } from "../store/app.svelte"
  import {
    claudeHookState, installClaudeHook, uninstallClaudeHook, type ClaudeHookState,
  } from "../ipc"
  import { alerts, setFlash, setToast } from "../store/alerts.svelte"
  import { t } from "../i18n/locale.svelte"

  /** Agent status settings: the Claude Code hook and how an agent that needs
   *  you gets your attention. The hook lives in the user's global
   *  ~/.claude/settings.json, so nothing is written until they have seen
   *  exactly what will be added and confirmed it. */
  let hook = $state<ClaudeHookState | null>(null)
  let confirming = $state(false)
  let busy = $state(false)

  async function refresh() {
    try {
      hook = await claudeHookState()
    } catch (e) {
      notify(t("settings.agents.readFailed", { error: String(e) }), "error")
    }
  }

  async function run(action: () => Promise<void>, done: string) {
    busy = true
    try {
      await action()
      notify(done)
    } catch (e) {
      notify(t("settings.agents.writeFailed", { error: String(e) }), "error")
    } finally {
      busy = false
      confirming = false
      await refresh()
    }
  }

  onMount(refresh)
</script>

<h3>{t("settings.agents.title")}</h3>
<p class="hint">{t("settings.agents.hint")}</p>

<div class="card">
  <span class="card-main">
    <strong>{t("settings.agents.claude")}</strong>
    <span class="state" class:ok={hook?.installed}>
      {hook === null ? "…" : hook.installed ? t("settings.agents.installed") : t("settings.agents.notInstalled")}
    </span>
  </span>
  {#if hook?.installed}
    <button
      class="ghost danger"
      disabled={busy}
      onclick={() => run(uninstallClaudeHook, t("settings.agents.removed"))}
    >{t("settings.agents.remove")}</button>
  {:else}
    <button class="ghost" disabled={busy || hook === null} onclick={() => (confirming = true)}>
      {t("settings.agents.install")}
    </button>
  {/if}
</div>

{#if confirming && hook}
  <div class="editor">
    <p class="hint">{t("settings.agents.confirm", { path: hook.path })}</p>
    <code class="block">{hook.events.join(", ")}</code>
    <code class="block">{hook.command}</code>
    <p class="hint">{t("settings.agents.backup")}</p>
    <div class="actions">
      <button class="ghost" disabled={busy} onclick={() => (confirming = false)}>
        {t("settings.cancel")}
      </button>
      <button
        class="primary"
        disabled={busy}
        onclick={() => run(installClaudeHook, t("settings.agents.added"))}
      >{t("settings.agents.install")}</button>
    </div>
  </div>
{/if}

<label class="card toggle">
  <input type="checkbox" checked={alerts.toast} onchange={(e) => setToast(e.currentTarget.checked)} />
  <span class="card-main"><strong>{t("settings.agents.toast")}</strong></span>
</label>
<label class="card toggle">
  <input type="checkbox" checked={alerts.flash} onchange={(e) => setFlash(e.currentTarget.checked)} />
  <span class="card-main"><strong>{t("settings.agents.flash")}</strong></span>
</label>

<style>
  .hint {
    margin: 0 0 14px;
    color: var(--text-dim);
    font-size: calc(12px * var(--font-scale, 1));
  }
  h3 {
    margin: 22px 0 8px;
    color: var(--text-dim);
    font-size: calc(11px * var(--font-scale, 1));
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .card {
    display: flex;
    gap: 8px;
    align-items: center;
    margin-bottom: 8px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-raised);
  }
  .card-main {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }
  .state {
    color: var(--text-dim);
    font-size: calc(11px * var(--font-scale, 1));
  }
  .state.ok {
    color: var(--ok);
  }
  .toggle {
    cursor: pointer;
  }
  .editor {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin-bottom: 8px;
    padding: 14px;
    border: 1px solid var(--accent);
    border-radius: 8px;
    background: var(--bg-raised);
  }
  .editor .hint {
    margin: 0;
  }
  .block {
    padding: 8px 10px;
    border-radius: 6px;
    background: var(--bg);
    color: var(--text);
    font-family: var(--font-mono);
    font-size: calc(11px * var(--font-scale, 1));
    word-break: break-all;
  }
  .actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }
  .ghost,
  .primary {
    padding: 6px 12px;
    border-radius: 6px;
    font: inherit;
    font-size: calc(12px * var(--font-scale, 1));
    cursor: pointer;
  }
  .ghost {
    border: 1px solid var(--border);
    background: none;
    color: var(--text-dim);
  }
  .ghost.danger {
    color: var(--err);
  }
  .ghost:disabled,
  .primary:disabled {
    cursor: default;
    opacity: 0.4;
  }
  .primary {
    border: 0;
    background: var(--accent);
    color: #fff;
  }
</style>
