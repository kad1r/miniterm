<script lang="ts">
  import { app, commit, notify } from "../store/app.svelte"
  import { pickDirectory } from "../ipc"
  import { insert } from "../store/tree"
  import { layoutsFor, MAX_TERMINALS, type GridLayout } from "../store/layout"
  import { pushRecent } from "../store/settings"
  import { emptyDraft, toWorkspace, basename, type Draft } from "../store/wizard"

  let { onclose }: { onclose: () => void } = $props()

  let draft = $state<Draft>(emptyDraft())
  let dialogEl = $state<HTMLDialogElement | undefined>(undefined)

  const layouts = $derived(layoutsFor(draft.count))
  // The directory is the only required field; everything else has a usable
  // default, which is what makes a single screen viable at all.
  const canCreate = $derived(draft.path.trim().length > 0)

  $effect(() => {
    if (dialogEl) {
      dialogEl.showModal()
    }
  })

  function choosePath(path: string) {
    const oldBasename = basename(draft.path)
    draft.path = path
    if (draft.name.trim() === "" || draft.name === oldBasename) draft.name = basename(path)
  }

  async function browse() {
    const picked = await pickDirectory()
    if (picked) choosePath(picked)
  }

  function setCount(n: number) {
    draft.count = n
    draft.layout = layoutsFor(n)[0]
  }

  function pickLayout(l: GridLayout) {
    draft.layout = l
  }

  function finish() {
    if (!canCreate) return
    const node = toWorkspace(draft)
    commit((c) => ({
      ...c,
      recentDirs: pushRecent(c.recentDirs, node.path),
      tree: insert(c.tree, node, { type: "rootEnd" }),
    }))
    app.activeWorkspaceId = node.id
    app.view = "workspace"
    notify(`"${node.name}" oluşturuldu`)
    onclose()
  }
</script>

<dialog
  bind:this={dialogEl}
  aria-modal="true"
  aria-label="Yeni workspace"
  oncancel={(e) => { e.preventDefault(); onclose() }}
>
  <div class="dialog-inner" role="presentation">
    <header>
      <h2>Yeni workspace</h2>
    </header>

    <div class="body">
      <section>
        <p class="q">Dizin</p>
        <div class="row">
          <input class="path" bind:value={draft.path} placeholder="C:\\projects\\api" />
          <button class="ghost" onclick={browse}>Gözat…</button>
        </div>

        {#if app.config.directories.length > 0}
          <p class="label">Kayıtlı dizinler</p>
          <div class="chips">
            {#each app.config.directories as d (d.id)}
              <button class="chip" title={d.path} onclick={() => choosePath(d.path)}>
                {d.alias}
              </button>
            {/each}
          </div>
        {/if}

        {#if app.config.recentDirs.length > 0}
          <p class="label">Son kullanılanlar</p>
          <div class="list">
            {#each app.config.recentDirs.slice(0, 5) as p (p)}
              <button class="recent" onclick={() => choosePath(p)}>{p}</button>
            {/each}
          </div>
        {/if}
      </section>

      <section>
        <p class="q">Ad ve shell</p>
        <div class="row">
          <input class="path" bind:value={draft.name} placeholder="workspace adı" />
          <select bind:value={draft.shellId}>
            <option value={null}>Varsayılan ({app.config.defaultShellId})</option>
            {#each app.shells as s (s.id)}
              <option value={s.id}>{s.name}</option>
            {/each}
          </select>
        </div>
      </section>

      <section>
        <p class="q">AI aracı</p>
        <div class="list">
          <button
            class="option"
            class:on={draft.aiToolId === null}
            onclick={() => (draft.aiToolId = null)}
          >
            <span class="opt-name">Sadece shell</span>
            <span class="opt-sub">Hiçbir komut çalıştırılmaz</span>
          </button>
          {#each app.config.aiTools as tool (tool.id)}
            <button
              class="option"
              class:on={draft.aiToolId === tool.id}
              onclick={() => (draft.aiToolId = tool.id)}
            >
              <span class="opt-name">{tool.name}</span>
              <span class="opt-sub">{tool.command}</span>
            </button>
          {/each}
        </div>
        <p class="note">Komut, workspace'teki <strong>her</strong> terminalde çalışır.</p>
      </section>

      <section>
        <p class="q">Terminaller</p>
        <div class="counts">
          {#each Array(MAX_TERMINALS) as _, i (i)}
            <button class="count" class:on={draft.count === i + 1} onclick={() => setCount(i + 1)}>
              {i + 1}
            </button>
          {/each}
        </div>

        <p class="label">Yerleşim</p>
        <div class="layouts">
          {#each layouts as l (`${l.rows}x${l.cols}`)}
            <button
              class="layout"
              class:on={draft.layout.rows === l.rows && draft.layout.cols === l.cols}
              onclick={() => pickLayout(l)}
            >
              <span
                class="mini"
                style="grid-template-columns:repeat({l.cols},1fr);
                       grid-template-rows:repeat({l.rows},1fr)"
              >
                {#each Array(l.rows * l.cols) as _, k (k)}<i></i>{/each}
              </span>
              <span class="layout-label">{l.rows}×{l.cols}</span>
            </button>
          {/each}
        </div>
      </section>
    </div>

    <footer>
      <button class="ghost" onclick={onclose}>Vazgeç</button>
      <span class="spacer"></span>
      <button class="primary" disabled={!canCreate} onclick={finish}>Oluştur</button>
    </footer>
  </div>
</dialog>

<style>
  dialog {
    padding: 0;
    border: none;
    border-radius: 10px;
    background: transparent;
    max-height: 80vh;
    max-width: 520px;
    width: 520px;
  }
  dialog::backdrop {
    background: rgb(0 0 0 / 0.5);
  }
  .dialog-inner {
    display: flex;
    flex-direction: column;
    width: 520px;
    max-height: 80vh;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 10px;
    overflow: hidden;
  }
  header {
    display: flex;
    align-items: center;
    padding: 14px 18px;
    border-bottom: 1px solid var(--border);
  }
  h2 {
    flex: 1;
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }
  .body {
    flex: 1;
    overflow-y: auto;
    padding: 16px 18px;
  }
  /* Separators carry the grouping the step counter used to. */
  section + section {
    margin-top: 18px;
    padding-top: 18px;
    border-top: 1px solid var(--border);
  }
  .q {
    margin: 0 0 12px;
    font-weight: 600;
    font-size: 13px;
  }
  .label {
    margin: 16px 0 6px;
    color: var(--text-dim);
    font-size: 11px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .row {
    display: flex;
    gap: 8px;
  }
  .path,
  select {
    flex: 1;
    width: 100%;
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: 13px;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chip {
    padding: 5px 10px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .recent,
  .option {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: 13px;
    text-align: left;
    cursor: pointer;
  }
  .option.on,
  .chip:hover,
  .recent:hover,
  .option:hover {
    border-color: var(--accent);
  }
  .opt-sub {
    color: var(--text-dim);
    font-family: var(--mono, monospace);
    font-size: 11px;
  }
  .note {
    margin-top: 14px;
    color: var(--text-dim);
    font-size: 12px;
  }
  .counts {
    display: flex;
    gap: 6px;
  }
  .count {
    width: 40px;
    height: 36px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg);
    color: var(--text);
    font: inherit;
    cursor: pointer;
  }
  .count.on {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 22%, transparent);
  }
  .layouts {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }
  .layout {
    display: flex;
    flex-direction: column;
    gap: 5px;
    align-items: center;
    padding: 8px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg);
    color: var(--text-dim);
    font: inherit;
    font-size: 11px;
    cursor: pointer;
  }
  .layout.on {
    border-color: var(--accent);
    color: var(--text);
  }
  .mini {
    display: grid;
    gap: 2px;
    width: 54px;
    height: 38px;
  }
  .mini i {
    background: var(--border);
    border-radius: 2px;
  }
  .layout.on .mini i {
    background: var(--accent);
  }
  footer {
    display: flex;
    gap: 8px;
    padding: 12px 18px;
    border-top: 1px solid var(--border);
  }
  .spacer {
    flex: 1;
  }
  .ghost,
  .primary {
    padding: 7px 14px;
    border-radius: 6px;
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .ghost {
    border: 1px solid var(--border);
    background: none;
    color: var(--text-dim);
  }
  .primary {
    border: 0;
    background: var(--accent);
    color: #fff;
  }
  .primary:disabled {
    opacity: 0.45;
    cursor: default;
  }
</style>
