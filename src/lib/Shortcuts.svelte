<script lang="ts">
  import { t } from "../i18n/locale.svelte"
  import type { MessageKey } from "../i18n/messages"

  let { onclose }: { onclose: () => void } = $props()

  let dialogEl = $state<HTMLDialogElement | undefined>(undefined)

  $effect(() => {
    if (dialogEl) dialogEl.showModal()
  })

  // Each row is a chord (rendered as <kbd> tokens) and the key that describes it.
  // The chords are literal, universal strings; only the description is translated.
  const groups: { title: MessageKey; rows: { keys: string[]; label: MessageKey }[] }[] = [
    {
      title: "shortcuts.group.workspace",
      rows: [
        { keys: ["Ctrl", "Shift", "N"], label: "shortcuts.newWorkspace" },
        { keys: ["Ctrl", "Shift", "T"], label: "shortcuts.newTerminal" },
        { keys: ["F2"], label: "shortcuts.rename" },
      ],
    },
    {
      title: "shortcuts.group.panes",
      rows: [
        { keys: ["Ctrl", "Tab"], label: "shortcuts.nextPane" },
        { keys: ["Ctrl", "Shift", "Tab"], label: "shortcuts.prevPane" },
        { keys: ["Ctrl", "Shift", "W"], label: "shortcuts.closePane" },
        { keys: ["Ctrl", "Shift", "M"], label: "shortcuts.minimizePane" },
        { keys: ["Ctrl", "Shift", "Z"], label: "shortcuts.maximizePane" },
      ],
    },
    {
      title: "shortcuts.group.view",
      rows: [
        { keys: ["Ctrl", ","], label: "shortcuts.settings" },
        { keys: ["Ctrl", "+"], label: "shortcuts.zoomIn" },
        { keys: ["Ctrl", "−"], label: "shortcuts.zoomOut" },
        { keys: ["Ctrl", "0"], label: "shortcuts.zoomReset" },
        { keys: ["F1"], label: "shortcuts.help" },
      ],
    },
  ]
</script>

<dialog
  bind:this={dialogEl}
  aria-modal="true"
  aria-label={t("shortcuts.title")}
  oncancel={(e) => { e.preventDefault(); onclose() }}
>
  <div class="dialog-inner" role="presentation">
    <header>
      <h2>{t("shortcuts.title")}</h2>
    </header>

    <div class="body">
      {#each groups as group (group.title)}
        <h3>{t(group.title)}</h3>
        <dl>
          {#each group.rows as row (row.label)}
            <div class="row">
              <dt>
                {#each row.keys as k, i (i)}
                  {#if i > 0}<span class="plus">+</span>{/if}
                  <kbd>{k}</kbd>
                {/each}
              </dt>
              <dd>{t(row.label)}</dd>
            </div>
          {/each}
        </dl>
      {/each}
      <p class="note">{t("shortcuts.passthrough")}</p>
    </div>

    <footer>
      <span class="spacer"></span>
      <button class="primary" onclick={onclose}>{t("shortcuts.close")}</button>
    </footer>
  </div>
</dialog>

<style>
  dialog {
    padding: 0;
    border: none;
    border-radius: 10px;
    background: transparent;
    width: min(460px, 92vw);
    max-width: 92vw;
  }
  dialog::backdrop {
    background: rgb(0 0 0 / 0.5);
  }
  .dialog-inner {
    display: flex;
    flex-direction: column;
    width: 100%;
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
    padding: 16px 18px;
    max-height: 70vh;
    overflow-y: auto;
  }
  h3 {
    margin: 14px 0 8px;
    color: var(--text-dim);
    font-size: calc(11px * var(--font-scale, 1));
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  h3:first-child {
    margin-top: 0;
  }
  dl {
    margin: 0;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 4px 0;
  }
  dt {
    display: flex;
    align-items: center;
    gap: 3px;
    flex: 0 0 auto;
    width: 128px;
  }
  dd {
    margin: 0;
    flex: 1;
    color: var(--text);
    font-size: calc(13px * var(--font-scale, 1));
  }
  kbd {
    display: inline-block;
    min-width: 12px;
    padding: 2px 6px;
    border: 1px solid var(--border);
    border-bottom-width: 2px;
    border-radius: 4px;
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: calc(11px * var(--font-scale, 1));
    text-align: center;
  }
  .plus {
    color: var(--text-dim);
    font-size: calc(11px * var(--font-scale, 1));
  }
  .note {
    margin: 16px 0 0;
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
  .primary {
    padding: 7px 14px;
    border: 0;
    border-radius: 6px;
    background: var(--accent);
    color: #fff;
    font: inherit;
    font-size: calc(13px * var(--font-scale, 1));
    cursor: pointer;
  }
</style>
