<script lang="ts">
  import { app, commit, notify } from "../store/app.svelte"
  import { pickDirectory } from "../ipc"
  import { insert } from "../store/tree"
  import { layoutsFor, MAX_TERMINALS, type GridLayout } from "../store/layout"
  import { pushRecent } from "../store/settings"
  import { emptyDraft, toWorkspace, basename, type Draft } from "../store/wizard"
  import AiLogo from "./AiLogo.svelte"
  import { t } from "../i18n/locale.svelte"

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
          <input class="path" bind:value={draft.path} placeholder="C:\\projects\\api" />
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
            onclick={() => (draft.aiToolId = null)}
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
              onclick={() => (draft.aiToolId = tool.id)}
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
        <p class="q">{t("wizard.terminals")}</p>
        <div class="counts">
          {#each Array(MAX_TERMINALS) as _, i (i)}
            <button class="count" class:on={draft.count === i + 1} onclick={() => setCount(i + 1)}>
              {i + 1}
            </button>
          {/each}
        </div>

        <p class="label">{t("wizard.layout")}</p>
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
    </div>

    <footer>
      <button class="ghost" onclick={onclose}>{t("settings.cancel")}</button>
      <span class="spacer"></span>
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
    max-height: 80vh;
    /* Wide enough for two columns on a normal window, still bounded by the
       viewport so a narrow window gets the single-column layout below. */
    width: min(920px, 92vw);
    max-width: 92vw;
  }
  dialog::backdrop {
    background: rgb(0 0 0 / 0.5);
  }
  .dialog-inner {
    display: flex;
    flex-direction: column;
    width: 100%;
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
    font-size: calc(15px * var(--font-scale, 1));
    font-weight: 600;
  }
  .body {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0 28px;
    align-items: start;
    flex: 1;
    overflow-y: auto;
    padding: 16px 18px;
  }
  /* Separators carry the grouping the step counter used to. Scoped to a column
     so the two columns do not draw a rule across each other's boundary. */
  section + section {
    margin-top: 18px;
    padding-top: 18px;
    border-top: 1px solid var(--border);
  }
  /* Below this width two columns leave each one too narrow for a directory
     path, so everything stacks and the column break becomes another rule. */
  @media (max-width: 820px) {
    .body {
      grid-template-columns: 1fr;
    }
    .col + .col {
      margin-top: 18px;
      padding-top: 18px;
      border-top: 1px solid var(--border);
    }
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
    font-size: calc(11px * var(--font-scale, 1));
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
