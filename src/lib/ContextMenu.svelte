<script lang="ts">
  interface Item {
    label: string
    action: () => void
    danger?: boolean
  }

  let { x, y, items, onclose }: {
    x: number
    y: number
    items: Item[]
    onclose: () => void
  } = $props()

  function pick(item: Item) {
    onclose()
    item.action()
  }
</script>

<svelte:window
  onkeydown={(e) => e.key === "Escape" && onclose()}
  onpointerdown={onclose}
/>

<div class="menu" style="left:{x}px; top:{y}px" role="menu">
  {#each items as item (item.label)}
    <button
      class="item"
      class:danger={item.danger}
      role="menuitem"
      onpointerdown={(e) => e.stopPropagation()}
      onclick={() => pick(item)}
    >
      {item.label}
    </button>
  {/each}
</div>

<style>
  .menu {
    position: fixed;
    z-index: 50;
    min-width: 170px;
    padding: 4px;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.45);
  }
  .item {
    display: block;
    width: 100%;
    padding: 6px 10px;
    border: 0;
    border-radius: 4px;
    background: none;
    color: var(--text);
    font: inherit;
    font-size: 13px;
    text-align: left;
    cursor: pointer;
  }
  .item:hover {
    background: color-mix(in srgb, var(--accent) 22%, transparent);
  }
  .item.danger {
    color: var(--err);
  }
</style>
