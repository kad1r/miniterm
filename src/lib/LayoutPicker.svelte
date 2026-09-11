<script lang="ts">
  import { layoutsFor, MAX_TERMINALS, type GridLayout } from "../store/layout"
  import { t } from "../i18n/locale.svelte"

  /** `counts` lets the caller narrow the offer — the new-workspace wizard allows
   *  every count, while adding a terminal to a live workspace cannot go below
   *  what is already open. */
  let { count, layout, counts = null, oncount, onlayout }: {
    count: number
    layout: GridLayout
    counts?: number[] | null
    oncount: (n: number) => void
    onlayout: (l: GridLayout) => void
  } = $props()

  const offered = $derived(counts ?? Array.from({ length: MAX_TERMINALS }, (_, i) => i + 1))
  const layouts = $derived(layoutsFor(count))
</script>

<p class="q">{t("wizard.terminals")}</p>
<div class="counts">
  {#each offered as n (n)}
    <button class="count" class:on={count === n} onclick={() => oncount(n)}>{n}</button>
  {/each}
</div>

<p class="label">{t("wizard.layout")}</p>
<div class="layouts">
  {#each layouts as l (`${l.rows}x${l.cols}`)}
    <button
      class="layout"
      class:on={layout.rows === l.rows && layout.cols === l.cols}
      onclick={() => onlayout(l)}
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

<style>
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
</style>
