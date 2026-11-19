<script lang="ts">
  import { app, commit, notify } from "../store/app.svelte"
  import { pickDirectory } from "../ipc"
  import { insert } from "../store/tree"
  import { layoutsFor, type GridLayout } from "../store/layout"
  import { pushRecent } from "../store/settings"
  import { emptyDraft, toWorkspace, basename, type Draft } from "../store/wizard"
  import { normalizeTools } from "../store/panes"
  import AiLogo from "./AiLogo.svelte"
  import LayoutPicker from "./LayoutPicker.svelte"
  import PaneToolPicker from "./PaneToolPicker.svelte"
  import { t } from "../i18n/locale.svelte"

  let { onclose }: { onclose: () => void } = $props()

  let draft = $state<Draft>(emptyDraft())
  let dialogEl = $state<HTMLDialogElement | undefined>(undefined)

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

  const cells = $derived(draft.layout.rows * draft.layout.cols)
  // The per-pane list is kept sparse in the draft and padded for display, so
  // adding a terminal or switching the workspace tool needs no bookkeeping here.
  const paneTools = $derived(normalizeTools(draft.paneTools, cells, draft.aiToolId))

  function setCount(n: number) {
    draft.count = n
    draft.layout = layoutsFor(n)[0]
  }

  function pickLayout(l: GridLayout) {
    draft.layout = l
  }

  /** Picking the workspace tool clears the per-pane choices. It is the coarse
   *  control: reaching for it means "all of them", and leaving overrides in
   *  place would make it look like it had done nothing. */
  function pickTool(id: string | null) {
    draft.aiToolId = id
    draft.paneTools = []
  }

  function pickPane(index: number, id: string | null) {
    draft.paneTools = paneTools.map((cur, i) => (i === index ? id : cur))
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
    notify(t("wizard.created", { name: node.name }))
    onclose()
  }
</script>

<dialog
  bind:this={dialogEl}
  aria-modal="true"
  aria-label={t("wizard.title")}
  oncancel={(e) => { e.preventDefault(); onclose() }}
>
  <div class="dialog-inner" role="presentation">
    <header>
      <h2>{t("wizard.title")}</h2>
    </header>

    <div class="body">
      <div class="col">
      <section>
        <p class="q">{t("wizard.dir")}</p>
        <div class="row">
          <input class="path" bind:value={draft.path} placeholder="C:\projects\api" />
          <button class="ghost" onclick={browse}>{t("settings.browse")}</button>
        </div>

        {#if app.config.directories.length > 0}
          <p class="label">{t("wizard.savedDirs")}</p>
          <div class="chips">
            {#each app.config.directories as d (d.id)}
              <button class="chip" title={d.path} onclick={() => choosePath(d.path)}>
                {d.alias}
              </button>
            {/each}
          </div>
        {/if}

        {#if app.config.recentDirs.length > 0}
          <p class="label">{t("wizard.recents")}</p>
          <div class="list">
            {#each app.config.recentDirs.slice(0, 5) as p (p)}
              <button class="recent" onclick={() => choosePath(p)}>{p}</button>
            {/each}
          </div>
        {/if}
      </section>

      <section>
        <p class="q">{t("wizard.nameAndShell")}</p>
        <div class="row">
          <input class="path" bind:value={draft.name} placeholder={t("wizard.namePlaceholder")} />
          <select bind:value={draft.shellId}>
            <option value={null}>
              {t("wizard.defaultShell", { shell: app.config.defaultShellId })}
            </option>
            {#each app.shells as s (s.id)}
              <option value={s.id}>{s.name}</option>
            {/each}
          </select>
        </div>
      </section>
      </div>

      <div class="col">
      <section>
        <p class="q">{t("wizard.aiTool")}</p>
        <div class="list">
          <button
            class="option"
            class:on={draft.aiToolId === null}
            onclick={() => pickTool(null)}
          >
            <span class="shell-mark" aria-hidden="true">›_</span>
            <span class="opt-text">
              <span class="opt-name">{t("wizard.shellOnly")}</span>
              <span class="opt-sub">{t("wizard.shellOnlySub")}</span>
            </span>
          </button>
          {#each app.config.aiTools as tool (tool.id)}
            <button
              class="option"
              class:on={draft.aiToolId === tool.id}
              onclick={() => pickTool(tool.id)}
            >
              <AiLogo command={tool.command} name={tool.name} />
              <span class="opt-text">
                <span class="opt-name">{tool.name}</span>
                <span class="opt-sub">{tool.command}</span>
              </span>
            </button>
          {/each}
        </div>
        <!-- The only message carrying inline markup. Safe to inject: it comes
             from our own static message table, never from user input. -->
        <p class="note">{@html t("wizard.note")}</p>
      </section>

      <section>
        <LayoutPicker
          count={draft.count}
          layout={draft.layout}
          oncount={setCount}
          onlayout={pickLayout}
        />
      </section>

      {#if cells > 1}
        <section>
          <p class="q">{t("panes.title")}</p>
          <PaneToolPicker tools={paneTools} onpick={pickPane} />
          <p class="note">{t("panes.hint")}</p>
        </section>
      {/if}
      </div>
    </div>

    <footer>
      <span class="spacer"></span>
      <button class="ghost" onclick={onclose}>{t("settings.cancel")}</button>
      <button class="primary" disabled={!canCreate} onclick={finish}>{t("wizard.create")}</button>
    </footer>
  </div>
</dialog>

<style>
  dialog {
    padding: 0;
    border: none;
    border-radius: 10px;
    background: transparent;
    max-height: 90vh;
    /* Wide enough for two columns, and grows with the UI font scale so zooming
       in does not squeeze the same content into the same pixels. Still bounded
       by the viewport: a window too narrow gets the columns stacked instead. */
    width: min(calc(920px * var(--font-scale, 1)), 92vw);
    max-width: 92vw;
  }
  dialog::backdrop {
    background: rgb(0 0 0 / 0.5);
  }
  .dialog-inner {
    display: flex;
    flex-direction: column;
    width: 100%;
    max-height: 90vh;
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
    font-size: calc(15px * var(--font-scale, 1));
    font-weight: 600;
  }
  /* Two columns that wrap into one when there is no room for both. A wrapping
     flex row rather than a grid with a width breakpoint: the room a column
     needs depends on the font scale, which a media query cannot see.
     `min-width: 0` matters as much as the wrap — without it a column refuses
     to get narrower than its longest unbreakable string (a deep directory
     path) and pushes its neighbour out of the dialog. */
  .body {
    display: flex;
    flex-wrap: wrap;
    gap: 22px 28px;
    align-items: flex-start;
    flex: 1;
    min-height: 0;
    overflow-x: hidden;
    overflow-y: auto;
    padding: 16px 18px;
  }
  .col {
    flex: 1 1 calc(320px * var(--font-scale, 1));
    min-width: 0;
  }
  /* Separators carry the grouping the step counter used to. Scoped to a column
     so the two columns do not draw a rule across each other's boundary. */
  section + section {
    margin-top: 18px;
    padding-top: 18px;
    border-top: 1px solid var(--border);
  }
  .q {
    margin: 0 0 12px;
    font-weight: 600;
    font-size: calc(13px * var(--font-scale, 1));
  }
  .label {
    margin: 16px 0 6px;
    color: var(--text-dim);
    font-size: calc(11px * var(--font-scale, 1));
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .row .ghost {
    flex: none;
  }
  .path,
  select {
    flex: 1 1 calc(140px * var(--font-scale, 1));
    min-width: 0;
    width: 100%;
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: calc(13px * var(--font-scale, 1));
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
    font-size: calc(12px * var(--font-scale, 1));
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
    gap: 2px;
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: calc(13px * var(--font-scale, 1));
    text-align: left;
    cursor: pointer;
    /* Paths and commands have no spaces to break at. */
    overflow-wrap: anywhere;
  }
  .recent {
    flex-direction: column;
  }
  /* The brand mark sits beside the label, so an AI-tool row runs horizontally
     while a recent directory stays a single stacked block. */
  .option {
    align-items: center;
    gap: 10px;
  }
  .opt-text {
    display: flex;
    min-width: 0;
    flex-direction: column;
    gap: 2px;
  }
  /* Stands in for a mark on the no-tool choice, at the same footprint so the
     labels of every option in the list line up. */
  .shell-mark {
    flex: none;
    width: 18px;
    color: var(--text-dim);
    font-family: var(--font-mono);
    font-size: calc(12px * var(--font-scale, 1));
    text-align: center;
  }
  .option.on,
  .chip:hover,
  .recent:hover,
  .option:hover {
    border-color: var(--accent);
  }
  .opt-sub {
    color: var(--text-dim);
    font-family: var(--font-mono);
    font-size: calc(11px * var(--font-scale, 1));
  }
  .note {
    margin-top: 14px;
    color: var(--text-dim);
    font-size: calc(12px * var(--font-scale, 1));
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
    font-size: calc(13px * var(--font-scale, 1));
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
