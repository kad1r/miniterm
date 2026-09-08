<script lang="ts">
  import type { Node } from "../store/types"
  import Self from "./TreeItem.svelte"

  let { node, depth, activeId, onselect, ontoggle, oncontext }: {
    node: Node
    depth: number
    activeId: string | null
    onselect: (node: Node) => void
    ontoggle: (node: Node) => void
    oncontext: (node: Node, x: number, y: number) => void
  } = $props()

  const isFolder = $derived(node.kind === "folder")
  const terminals = $derived(node.kind === "workspace" ? node.rows * node.cols : 0)
</script>

<div
  class="tree-item"
  class:active={node.id === activeId}
  data-node-id={node.id}
  data-kind={node.kind}
  style="padding-left:{4 + (depth - 1) * 14}px"
  role="treeitem"
  aria-selected={node.id === activeId}
  aria-expanded={node.kind === "folder" ? node.expanded : undefined}
  tabindex="0"
  onclick={() => (isFolder ? ontoggle(node) : onselect(node))}
  onkeydown={(e) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault()
      if (isFolder) ontoggle(node)
      else onselect(node)
    }
  }}
  oncontextmenu={(e) => {
    e.preventDefault()
    oncontext(node, e.clientX, e.clientY)
  }}
>
  <span class="chevron" class:hidden={!isFolder}>
    {node.kind === "folder" && node.expanded ? "▾" : "▸"}
  </span>
  <span class="name">{node.name}</span>
  {#if !isFolder}
    <span class="badge">{terminals}</span>
  {/if}
</div>

{#if node.kind === "folder" && node.expanded}
  {#each node.children as child (child.id)}
    <Self node={child} depth={depth + 1} {activeId} {onselect} {ontoggle} {oncontext} />
  {/each}
{/if}

<style>
  .tree-item {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 26px;
    padding-right: 6px;
    border-radius: 4px;
    color: var(--text);
    font-size: 13px;
    cursor: pointer;
    user-select: none;
  }
  .tree-item:hover {
    background: color-mix(in srgb, var(--text) 8%, transparent);
  }
  .tree-item.active {
    background: color-mix(in srgb, var(--accent) 26%, transparent);
  }
  .chevron {
    width: 12px;
    color: var(--text-dim);
    font-size: 10px;
  }
  .chevron.hidden {
    visibility: hidden;
  }
  .name {
    flex: 1;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .badge {
    color: var(--text-dim);
    font-size: 11px;
  }
</style>
