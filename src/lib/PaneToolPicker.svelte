<script lang="ts">
  import { app } from "../store/app.svelte"
  import type { PaneTools } from "../store/panes"
  import { t } from "../i18n/locale.svelte"

  /** One row per cell of the grid: which AI tool that terminal opens with.
   *
   *  `lockedBefore` marks the cells that already have a shell running — their
   *  command was typed into that shell when it started, so changing the setting
   *  now would say one thing and show another. The dialog that adds terminals
   *  passes the current count; the wizard, where nothing is running yet,
   *  passes nothing. */
  let { tools, lockedBefore = 0, onpick }: {
    tools: PaneTools
    lockedBefore?: number
    onpick: (index: number, toolId: string | null) => void
  } = $props()

  // A select's value is a string, so the "no tool" choice needs a sentinel. The
  // empty string cannot collide with a tool id: those are UUIDs.
  const NONE = ""
</script>

<div class="panes">
  {#each tools as toolId, i (i)}
    <label class="pane" class:locked={i < lockedBefore}>
      <span class="name">
        {t("panes.pane", { n: i + 1 })}
        {#if i < lockedBefore}<span class="tag">{t("panes.running")}</span>{/if}
      </span>
      <select
        value={toolId ?? NONE}
        disabled={i < lockedBefore}
        onchange={(e) => onpick(i, e.currentTarget.value === NONE ? null : e.currentTarget.value)}
      >
        <option value={NONE}>{t("wizard.shellOnly")}</option>
        {#each app.config.aiTools as tool (tool.id)}
          <option value={tool.id}>{tool.name}</option>
        {/each}
      </select>
    </label>
  {/each}
</div>

<style>
  .panes {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .pane {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .name {
    display: flex;
    align-items: baseline;
    gap: 6px;
    flex: 1;
    min-width: 0;
    font-size: calc(13px * var(--font-scale, 1));
  }
  .locked .name {
    color: var(--text-dim);
  }
  .tag {
    color: var(--text-dim);
    font-size: calc(11px * var(--font-scale, 1));
  }
  select {
    flex: 1;
    min-width: 0;
    padding: 6px 8px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: calc(13px * var(--font-scale, 1));
  }
  select:disabled {
    opacity: 0.5;
  }
</style>
