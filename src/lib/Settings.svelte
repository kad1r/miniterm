<script lang="ts">
  import { app, commit, notify } from "../store/app.svelte"
  import { detectShells, pickDirectory } from "../ipc"
  import {
    commandPreview, removeDir, removeTool, toolUsage, upsertDir, upsertTool,
  } from "../store/settings"
  import type { AiTool, DirAlias } from "../store/types"
  import { DEFAULT_FONT_SCALE, MAX_FONT_SCALE, MIN_FONT_SCALE } from "../store/font"
  import { applyFontAction, fontScale, setFontScale } from "../store/font.svelte"
  import { LOCALES, LOCALE_NAMES, type Locale } from "../i18n/messages"
  import { locale, setLocale, t } from "../i18n/locale.svelte"

  let tab = $state<"tools" | "dirs" | "terminal" | "gorunum">("tools")
  let editingTool = $state<AiTool | null>(null)
  let editingDir = $state<DirAlias | null>(null)

  const defaultShell = $derived(
    app.shells.find((s) => s.id === app.config.defaultShellId) ?? null,
  )

  function newTool() {
    editingTool = { id: crypto.randomUUID(), name: "", command: "" }
  }

  function saveTool() {
    const tool = editingTool
    if (!tool || tool.name.trim() === "" || tool.command.trim() === "") {
      notify(t("settings.tools.required"), "error")
      return
    }
    commit((c) => upsertTool(c, { ...tool, name: tool.name.trim(), command: tool.command.trim() }))
    editingTool = null
  }

  function deleteTool(tool: AiTool) {
    const used = toolUsage(app.config.tree, tool.id)
    const warning =
      used === 0
        ? t("settings.tools.deleteConfirm", { name: tool.name })
        : t("settings.tools.deleteConfirmUsed", { name: tool.name, count: used })
    if (!confirm(warning)) return
    commit((c) => removeTool(c, tool.id))
    notify(t("sidebar.deleted", { name: tool.name }))
  }

  function newDir() {
    editingDir = { id: crypto.randomUUID(), alias: "", path: "" }
  }

  async function browseDir() {
    const picked = await pickDirectory()
    if (picked && editingDir) {
      editingDir.path = picked
      if (editingDir.alias.trim() === "") {
        editingDir.alias = picked.replace(/[\\/]+$/, "").split(/[\\/]/).pop() ?? ""
      }
    }
  }

  function saveDir() {
    const dir = editingDir
    if (!dir || dir.alias.trim() === "" || dir.path.trim() === "") {
      notify(t("settings.dirs.required"), "error")
      return
    }
    commit((c) => upsertDir(c, { ...dir, alias: dir.alias.trim(), path: dir.path.trim() }))
    editingDir = null
  }

  function deleteDir(dir: DirAlias) {
    if (!confirm(t("settings.dirs.deleteConfirm", { alias: dir.alias }))) return
    commit((c) => removeDir(c, dir.id))
  }

  async function rescan() {
    app.shells = await detectShells()
    if (!app.shells.some((s) => s.id === app.config.defaultShellId) && app.shells[0]) {
      const first = app.shells[0].id
      commit((c) => ({ ...c, defaultShellId: first }))
    }
    notify(t("settings.terminal.found", { count: app.shells.length }))
  }

  function setDefaultShell(id: string) {
    commit((c) => ({ ...c, defaultShellId: id }))
  }
</script>

<section class="settings">
  <nav>
    <button class:on={tab === "tools"} onclick={() => (tab = "tools")}>
      {t("settings.tab.tools")}
    </button>
    <button class:on={tab === "dirs"} onclick={() => (tab = "dirs")}>
      {t("settings.tab.dirs")}
    </button>
    <button class:on={tab === "terminal"} onclick={() => (tab = "terminal")}>
      {t("settings.tab.terminal")}
    </button>
    <button class:on={tab === "gorunum"} onclick={() => (tab = "gorunum")}>
      {t("settings.tab.appearance")}
    </button>
  </nav>

  <div class="content">
    {#if tab === "tools"}
      <p class="hint">{t("settings.tools.hint")}</p>

      {#each app.config.aiTools as tool (tool.id)}
        <div class="card">
          <div class="card-main">
            <strong>{tool.name}</strong>
            <code>{tool.command}</code>
            <span class="preview">
              {commandPreview(defaultShell, tool.command, locale.current)}
            </span>
          </div>
          <button class="ghost" onclick={() => (editingTool = { ...tool })}>
            {t("settings.edit")}
          </button>
          <button class="ghost danger" onclick={() => deleteTool(tool)}>
            {t("settings.delete")}
          </button>
        </div>
      {/each}

      {#if editingTool}
        {@const tool = editingTool}
        <div class="editor">
          <label>
            {t("settings.tools.name")}<input bind:value={tool.name} placeholder="Claude Code" />
          </label>
          <label>
            {t("settings.tools.command")}
            <input bind:value={tool.command} placeholder="claude --model sonnet-5" />
          </label>
          <p class="preview">{commandPreview(defaultShell, tool.command, locale.current)}</p>
          <div class="actions">
            <button class="ghost" onclick={() => (editingTool = null)}>
              {t("settings.cancel")}
            </button>
            <button class="primary" onclick={saveTool}>{t("settings.save")}</button>
          </div>
        </div>
      {:else}
        <button class="ghost add" onclick={newTool}>{t("settings.tools.add")}</button>
      {/if}

    {:else if tab === "dirs"}
      {#each app.config.directories as dir (dir.id)}
        <div class="card">
          <div class="card-main">
            <strong>{dir.alias}</strong>
            <code>{dir.path}</code>
          </div>
          <button class="ghost" onclick={() => (editingDir = { ...dir })}>
            {t("settings.edit")}
          </button>
          <button class="ghost danger" onclick={() => deleteDir(dir)}>
            {t("settings.delete")}
          </button>
        </div>
      {/each}

      {#if editingDir}
        {@const dir = editingDir}
        <div class="editor">
          <label>
            {t("settings.dirs.alias")}<input bind:value={dir.alias} placeholder="api" />
          </label>
          <label>
            {t("settings.dirs.path")}
            <span class="row">
              <input bind:value={dir.path} placeholder="C:\\projects\\api" />
              <button class="ghost" onclick={browseDir}>{t("settings.browse")}</button>
            </span>
          </label>
          <div class="actions">
            <button class="ghost" onclick={() => (editingDir = null)}>
              {t("settings.cancel")}
            </button>
            <button class="primary" onclick={saveDir}>{t("settings.save")}</button>
          </div>
        </div>
      {:else}
        <button class="ghost add" onclick={newDir}>{t("settings.dirs.add")}</button>
      {/if}

      {#if app.config.recentDirs.length > 0}
        <h3>{t("settings.dirs.recents")}</h3>
        <ul class="recents">
          {#each app.config.recentDirs as p (p)}
            <li>{p}</li>
          {/each}
        </ul>
      {/if}

    {:else if tab === "terminal"}
      <p class="hint">{t("settings.terminal.hint")}</p>
      {#each app.shells as shell (shell.id)}
        <label class="card shell">
          <input
            type="radio"
            name="default-shell"
            checked={app.config.defaultShellId === shell.id}
            onchange={() => setDefaultShell(shell.id)}
          />
          <span class="card-main">
            <strong>{shell.name}</strong>
            <code>{[shell.program, ...shell.args].join(" ")}</code>
          </span>
        </label>
      {/each}
      <button class="ghost add" onclick={rescan}>{t("settings.terminal.rescan")}</button>

    {:else}
      <div class="card">
        <span class="card-main"><strong>{t("settings.appearance.language")}</strong></span>
        {#each LOCALES as code (code)}
          <button
            class="ghost"
            class:on={locale.current === code}
            aria-pressed={locale.current === code}
            onclick={() => setLocale(code as Locale)}
          >{LOCALE_NAMES[code]}</button>
        {/each}
      </div>

      <p class="hint">{t("settings.appearance.hint")}</p>
      <div class="card font-scale">
        <span class="card-main"><strong>{t("settings.appearance.fontSize")}</strong></span>
        <button
          class="ghost step"
          aria-label={t("settings.appearance.smaller")}
          disabled={fontScale.level <= MIN_FONT_SCALE}
          onclick={() => applyFontAction("out")}
        >−</button>
        <output class="level">%{Math.round(fontScale.level * 100)}</output>
        <button
          class="ghost step"
          aria-label={t("settings.appearance.larger")}
          disabled={fontScale.level >= MAX_FONT_SCALE}
          onclick={() => applyFontAction("in")}
        >+</button>
        <button
          class="ghost"
          disabled={fontScale.level === DEFAULT_FONT_SCALE}
          onclick={() => setFontScale(DEFAULT_FONT_SCALE)}
        >{t("settings.appearance.reset")}</button>
      </div>
    {/if}
  </div>
</section>

<style>
  .settings {
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  nav {
    display: flex;
    gap: 2px;
    padding: 0 16px;
    border-bottom: 1px solid var(--border);
  }
  nav button {
    padding: 11px 14px;
    border: 0;
    border-bottom: 2px solid transparent;
    background: none;
    color: var(--text-dim);
    font: inherit;
    font-size: calc(13px * var(--font-scale, 1));
    cursor: pointer;
  }
  nav button.on {
    border-bottom-color: var(--accent);
    color: var(--text);
  }
  .content {
    flex: 1;
    overflow-y: auto;
    max-width: 640px;
    padding: 18px 16px 40px;
  }
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
  .card-main code,
  .preview {
    overflow: hidden;
    color: var(--text-dim);
    font-size: calc(11px * var(--font-scale, 1));
    text-overflow: ellipsis;
    white-space: nowrap;
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
  label {
    display: flex;
    flex-direction: column;
    gap: 5px;
    color: var(--text-dim);
    font-size: calc(12px * var(--font-scale, 1));
  }
  .row {
    display: flex;
    gap: 8px;
  }
  input:not([type]) {
    width: 100%;
    padding: 7px 9px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: calc(13px * var(--font-scale, 1));
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
  .ghost.on {
    border-color: var(--accent);
    color: var(--text);
  }
  .ghost.add {
    width: 100%;
    margin-top: 4px;
    padding: 9px;
  }
  .primary {
    border: 0;
    background: var(--accent);
    color: #fff;
  }
  .shell {
    flex-direction: row;
    align-items: center;
    cursor: pointer;
  }
  .ghost:disabled {
    cursor: default;
    opacity: 0.4;
  }
  .step {
    width: 30px;
    font-size: calc(15px * var(--font-scale, 1));
    line-height: 1;
  }
  /* Fixed width so the row does not jitter as the percentage changes width. */
  .level {
    width: 46px;
    color: var(--text-dim);
    font-size: calc(12px * var(--font-scale, 1));
    text-align: center;
  }
  .recents {
    margin: 0;
    padding-left: 18px;
    color: var(--text-dim);
    font-size: calc(12px * var(--font-scale, 1));
    line-height: 1.7;
  }
</style>
