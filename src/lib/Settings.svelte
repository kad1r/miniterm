<script lang="ts">
  import changelogSource from "../../CHANGELOG.md?raw"
  import { app, commit, notify } from "../store/app.svelte"
  import { appVersion, detectShells, pickDirectory } from "../ipc"
  import { groupBlocks, parseChangelog, releaseFor, type Span } from "../store/changelog"
  import {
    commandPreview, removeDir, removeTool, toolUsage, upsertDir, upsertTool,
  } from "../store/settings"
  import type { AiTool, DirAlias } from "../store/types"
  import { DEFAULT_FONT_SCALE, MAX_FONT_SCALE, MIN_FONT_SCALE } from "../store/font"
  import { applyFontAction, fontScale, setFontScale } from "../store/font.svelte"
  import { setTheme, theme } from "../store/theme.svelte"
  import { update, runCheck, runDownload, runInstall } from "../store/update.svelte"
  import { LOCALES, LOCALE_NAMES, type Locale } from "../i18n/messages"
  import { locale, setLocale, t } from "../i18n/locale.svelte"

  let tab = $state<"tools" | "dirs" | "terminal" | "gorunum" | "about">("tools")
  let editingTool = $state<AiTool | null>(null)
  let editingDir = $state<DirAlias | null>(null)

  const REPO = "https://github.com/kad1r/miniterm"

  async function checkForUpdates() {
    await runCheck(false)
  }
  async function downloadAndInstall() {
    const path = await runDownload()
    if (path && confirm(t("update.installConfirm"))) await runInstall(path)
  }

  // Parsed once at module-eval cost, not per tab switch: the file is a build
  // constant and cannot change while the app runs.
  const releases = parseChangelog(changelogSource)

  // Empty until the bundle answers. Nothing is rendered from it in the meantime,
  // so there is no flash of a wrong version.
  let version = $state("")
  appVersion().then((v) => (version = v)).catch(() => {})

  const release = $derived(version === "" ? null : releaseFor(releases, version))

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
    <button class:on={tab === "about"} onclick={() => (tab = "about")}>
      {t("settings.tab.about")}
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

    {:else if tab === "gorunum"}
      <div class="card">
        <span class="card-main"><strong>{t("settings.appearance.theme")}</strong></span>
        <button
          class="ghost"
          class:on={theme.current === "dark"}
          aria-pressed={theme.current === "dark"}
          onclick={() => setTheme("dark")}
        >{t("settings.appearance.themeDark")}</button>
        <button
          class="ghost"
          class:on={theme.current === "light"}
          aria-pressed={theme.current === "light"}
          onclick={() => setTheme("light")}
        >{t("settings.appearance.themeLight")}</button>
      </div>

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

    {:else}
      <div class="card about">
        <span class="card-main">
          <strong class="product">miniterm</strong>
          <span class="tagline">{t("settings.about.tagline")}</span>
        </span>
        <span class="version">
          <span class="label">{t("settings.about.version")}</span>
          <output>{version === "" ? t("settings.about.unknownVersion") : version}</output>
          {#if release?.date}
            <small>{t("settings.about.released", { date: release.date })}</small>
          {/if}
        </span>
      </div>

      <h3>{t("settings.about.notes")}</h3>
      {#if release}
        <div class="notes">
          {#each groupBlocks(release.blocks) as group, i (i)}
            {#if group.kind === "heading"}
              <h4>{@render runs(group.block.spans)}</h4>
            {:else if group.kind === "para"}
              <p>{@render runs(group.block.spans)}</p>
            {:else if group.kind === "list"}
              <ul>
                {#each group.items as item, j (j)}
                  <li>{@render runs(item.spans)}</li>
                {/each}
              </ul>
            {/if}
          {/each}
        </div>
      {:else}
        <p class="hint">{t("settings.about.noNotes", { version })}</p>
      {/if}

      <h3>{t("settings.about.source")}</h3>
      <p class="repo"><code>{REPO}</code></p>

      <h3>{t("update.checkButton")}</h3>
      <div class="update-check">
        <button
          onclick={checkForUpdates}
          disabled={update.status === "checking" || update.status === "downloading"}
        >
          {update.status === "checking" ? t("update.checking") : t("update.checkButton")}
        </button>
        {#if update.status === "upToDate"}
          <span class="hint">{t("update.upToDate")}</span>
        {:else if update.status === "error"}
          <span class="hint err">{t("update.failed")}</span>
        {:else if update.status === "available"}
          <span class="hint">{t("update.bannerAvailable", { version: update.info?.latest ?? "" })}</span>
          <button onclick={downloadAndInstall}>{t("update.download")}</button>
        {:else if update.status === "downloading"}
          <span class="hint">{t("update.downloading", { percent: update.progress?.total ? Math.floor((update.progress.downloaded / update.progress.total) * 100) : 0 })}</span>
        {:else if update.status === "ready"}
          <span class="hint">{t("update.installReady")}</span>
        {/if}
      </div>
    {/if}
  </div>
</section>

<!-- One line on purpose: a newline between the branches would be rendered as a
     space inside the sentence, so "pick **Apply** now" would gain gaps. -->
{#snippet runs(spans: Span[])}{#each spans as s, i (i)}{#if s.style === "bold"}<strong>{s.text}</strong>{:else if s.style === "code"}<code>{s.text}</code>{:else}{s.text}{/if}{/each}{/snippet}

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
  .update-check {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }
  .update-check .hint {
    margin: 0;
  }
  .update-check .hint.err {
    color: var(--err);
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
  .about {
    align-items: flex-start;
    padding: 14px;
  }
  .product {
    font-size: calc(16px * var(--font-scale, 1));
  }
  .tagline {
    color: var(--text-dim);
    font-size: calc(12px * var(--font-scale, 1));
    white-space: normal;
  }
  .version {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 2px;
    text-align: right;
  }
  .version .label {
    color: var(--text-dim);
    font-size: calc(11px * var(--font-scale, 1));
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .version output {
    font-size: calc(17px * var(--font-scale, 1));
    font-variant-numeric: tabular-nums;
  }
  .version small {
    color: var(--text-dim);
    font-size: calc(11px * var(--font-scale, 1));
  }
  .notes {
    font-size: calc(12px * var(--font-scale, 1));
    line-height: 1.65;
  }
  .notes h4 {
    margin: 16px 0 6px;
    font-size: calc(13px * var(--font-scale, 1));
  }
  .notes h4:first-child {
    margin-top: 0;
  }
  .notes p {
    margin: 0 0 8px;
    color: var(--text-dim);
  }
  .notes ul {
    margin: 0 0 8px;
    padding-left: 18px;
    color: var(--text-dim);
  }
  .notes strong {
    color: var(--text);
  }
  .notes code,
  .repo code {
    padding: 1px 4px;
    border-radius: 4px;
    background: var(--bg-raised);
    font-size: calc(11px * var(--font-scale, 1));
  }
  .repo {
    margin: 0;
    color: var(--text-dim);
    user-select: text;
  }
</style>
