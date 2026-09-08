<script lang="ts">
  import type { Node } from "../store/types"
  import type { DropZone } from "../store/dnd"
  import Self from "./TreeItem.svelte"

  let { node, depth, activeId, dragId, dropHint, onselect, ontoggle, oncontext,
        ondragstart, ondragover, ondrop, ondragend, statusFor }: {
    node: Node
    depth: number
    activeId: string | null
    dragId: string | null
    dropHint: { nodeId: string; zone: DropZone; ok: boolean } | null
    onselect: (node: Node) => void
    ontoggle: (node: Node) => void
    oncontext: (node: Node, x: number, y: number) => void
    ondragstart: (node: Node) => void
    ondragover: (node: Node, offsetY: number, height: number) => void
    ondrop: () => void
    ondragend: () => void
    statusFor: (nodeId: string) => "off" | "running" | "dead"
  } = $props()

  const isFolder = $derived(node.kind === "folder")
  const terminals = $derived(node.kind === "workspace" ? node.rows * node.cols : 0)
  const hint = $derived(dropHint?.nodeId === node.id ? dropHint : null)
  const status = $derived(node.kind === "workspace" ? statusFor(node.id) : "off")
</script>

<div
  class="tree-item"
  class:active={node.id === activeId}
  class:dragging={node.id === dragId}
  class:hint-before={hint?.zone === "before" && hint.ok}
  class:hint-after={hint?.zone === "after" && hint.ok}
  class:hint-into={hint?.zone === "into" && hint.ok}
  class:hint-bad={hint != null && !hint.ok}
  data-node-id={node.id}
  data-kind={node.kind}
  style="padding-left:{4 + (depth - 1) * 14}px"
  role="treeitem"
  aria-selected={node.id === activeId}
  aria-expanded={node.kind === "folder" ? node.expanded : undefined}
  tabindex="0"
  draggable="true"
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
  ondragstart={(e) => {
    e.stopPropagation()
    e.dataTransfer?.setData("text/plain", node.id)
    ondragstart(node)
  }}
  ondragover={(e) => {
    e.preventDefault()
    e.stopPropagation()
    const box = (e.currentTarget as HTMLElement).getBoundingClientRect()
    ondragover(node, e.clientY - box.top, box.height)
    if (e.dataTransfer) e.dataTransfer.dropEffect = hint?.ok === false ? "none" : "move"
  }}
  ondrop={(e) => {
    e.preventDefault()
    e.stopPropagation()
    ondrop()
  }}
  ondragend={ondragend}
>
  <span class="chevron" class:hidden={!isFolder}>
    {node.kind === "folder" && node.expanded ? "▾" : "▸"}
  </span>
  <span class="name">{node.name}</span>
  {#if !isFolder}
    <span class="dot {status}" title={status}></span>
  {/if}
  {#if !isFolder}
    <span class="badge">{terminals}</span>
  {/if}
</div>

{#if node.kind === "folder" && node.expanded}
  {#each node.children as child (child.id)}
    <Self node={child} depth={depth + 1} {activeId} {dragId} {dropHint}
          {onselect} {ontoggle} {oncontext}
          {ondragstart} {ondragover} {ondrop} {ondragend} {statusFor} />
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
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--text-dim);
  }
  .dot.running {
    background: var(--ok);
  }
  .dot.dead {
    background: var(--err);
  }
  .badge {
    color: var(--text-dim);
    font-size: 11px;
  }
  .tree-item.dragging {
    opacity: 0.4;
  }
  .tree-item.hint-before {
    box-shadow: inset 0 2px 0 var(--accent);
  }
  .tree-item.hint-after {
    box-shadow: inset 0 -2px 0 var(--accent);
  }
  .tree-item.hint-into {
    outline: 1px solid var(--accent);
    outline-offset: -1px;
  }
  .tree-item.hint-bad {
    outline: 1px solid var(--err);
    outline-offset: -1px;
    cursor: not-allowed;
  }
</style>
