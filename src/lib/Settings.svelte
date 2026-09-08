<script lang="ts">
  import { app, commit, notify } from "../store/app.svelte"
  import { detectShells, pickDirectory } from "../ipc"
  import {
    commandPreview, removeDir, removeTool, toolUsage, upsertDir, upsertTool,
  } from "../store/settings"
  import type { AiTool, DirAlias } from "../store/types"

  let tab = $state<"tools" | "dirs" | "terminal">("tools")
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
      notify("Ad ve komut boş olamaz", "error")
      return
    }
    commit((c) => upsertTool(c, { ...tool, name: tool.name.trim(), command: tool.command.trim() }))
    editingTool = null
  }

  function deleteTool(tool: AiTool) {
    const used = toolUsage(app.config.tree, tool.id)
    const warning =
      used === 0
        ? `"${tool.name}" silinecek. Devam?`
        : `"${tool.name}" silinecek. ${used} workspace "Sadece shell"e dönecek. Devam?`
    if (!confirm(warning)) return
    commit((c) => removeTool(c, tool.id))
    notify(`"${tool.name}" silindi`)
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
      notify("Alias ve yol boş olamaz", "error")
      return
    }
    commit((c) => upsertDir(c, { ...dir, alias: dir.alias.trim(), path: dir.path.trim() }))
    editingDir = null
  }

  function deleteDir(dir: DirAlias) {
    if (!confirm(`"${dir.alias}" kısayolu silinecek. Workspace'ler etkilenmez. Devam?`)) return
    commit((c) => removeDir(c, dir.id))
  }

  async function rescan() {
    app.shells = await detectShells()
    if (!app.shells.some((s) => s.id === app.config.defaultShellId) && app.shells[0]) {
      const first = app.shells[0].id
      commit((c) => ({ ...c, defaultShellId: first }))
    }
    notify(`${app.shells.length} shell bulundu`)
  }

  function setDefaultShell(id: string) {
    commit((c) => ({ ...c, defaultShellId: id }))
  }
</script>

<section class="settings">
  <nav>
    <button class:on={tab === "tools"} onclick={() => (tab = "tools")}>AI Araçları</button>
    <button class:on={tab === "dirs"} onclick={() => (tab = "dirs")}>Dizinler</button>
    <button class:on={tab === "terminal"} onclick={() => (tab = "terminal")}>Terminal</button>
  </nav>

  <div class="content">
    {#if tab === "tools"}
      <p class="hint">
        miniterm hiçbir kimlik bilgisi saklamaz. Oturum açma işini AI CLI'ları kendi yapar.
      </p>

      {#each app.config.aiTools as tool (tool.id)}
        <div class="card">
          <div class="card-main">
            <strong>{tool.name}</strong>
            <code>{tool.command}</code>
            <span class="preview">{commandPreview(defaultShell, tool.command)}</span>
          </div>
          <button class="ghost" onclick={() => (editingTool = { ...tool })}>Düzenle</button>
          <button class="ghost danger" onclick={() => deleteTool(tool)}>Sil</button>
        </div>
      {/each}

      {#if editingTool}
        {@const tool = editingTool}
        <div class="editor">
          <label>Ad<input bind:value={tool.name} placeholder="Claude Code" /></label>
          <label>Komut<input bind:value={tool.command} placeholder="claude --model sonnet-5" /></label>
          <p class="preview">{commandPreview(defaultShell, tool.command)}</p>
          <div class="actions">
            <button class="ghost" onclick={() => (editingTool = null)}>Vazgeç</button>
            <button class="primary" onclick={saveTool}>Kaydet</button>
          </div>
        </div>
      {:else}
        <button class="ghost add" onclick={newTool}>+ AI aracı ekle</button>
      {/if}

    {:else if tab === "dirs"}
      {#each app.config.directories as dir (dir.id)}
        <div class="card">
          <div class="card-main">
            <strong>{dir.alias}</strong>
            <code>{dir.path}</code>
          </div>
          <button class="ghost" onclick={() => (editingDir = { ...dir })}>Düzenle</button>
          <button class="ghost danger" onclick={() => deleteDir(dir)}>Sil</button>
        </div>
      {/each}

      {#if editingDir}
        {@const dir = editingDir}
        <div class="editor">
          <label>Alias<input bind:value={dir.alias} placeholder="api" /></label>
          <label>
            Yol
            <span class="row">
              <input bind:value={dir.path} placeholder="C:\\projects\\api" />
              <button class="ghost" onclick={browseDir}>Gözat…</button>
            </span>
          </label>
          <div class="actions">
            <button class="ghost" onclick={() => (editingDir = null)}>Vazgeç</button>
            <button class="primary" onclick={saveDir}>Kaydet</button>
          </div>
        </div>
      {:else}
        <button class="ghost add" onclick={newDir}>+ Dizin kısayolu ekle</button>
      {/if}

      {#if app.config.recentDirs.length > 0}
        <h3>Son kullanılanlar</h3>
        <ul class="recents">
          {#each app.config.recentDirs as p (p)}
            <li>{p}</li>
          {/each}
        </ul>
      {/if}

    {:else}
      <p class="hint">Varsayılan shell, kendi shell'ini seçmemiş workspace'ler için kullanılır.</p>
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
      <button class="ghost add" onclick={rescan}>Shell'leri yeniden tara</button>
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
    font-size: 13px;
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
    font-size: 12px;
  }
  h3 {
    margin: 22px 0 8px;
    color: var(--text-dim);
    font-size: 11px;
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
    font-size: 11px;
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
    font-size: 12px;
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
    font-size: 13px;
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
    font-size: 12px;
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
  .recents {
    margin: 0;
    padding-left: 18px;
    color: var(--text-dim);
    font-size: 12px;
    line-height: 1.7;
  }
</style>
