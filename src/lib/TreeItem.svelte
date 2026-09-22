<script lang="ts">
  import type { Node } from "../store/types"
  import type { DropZone } from "../store/dnd"
  import { t } from "../i18n/locale.svelte"
  import Self from "./TreeItem.svelte"

  let { node, depth, activeId, dragId, dropHint, onselect, ontoggle, oncontext,
        onpointerdownitem, statusFor }: {
    node: Node
    depth: number
    activeId: string | null
    dragId: string | null
    dropHint: { nodeId: string; zone: DropZone; ok: boolean } | null
    onselect: (node: Node) => void
    ontoggle: (node: Node) => void
    oncontext: (node: Node, x: number, y: number) => void
    onpointerdownitem: (node: Node, e: PointerEvent) => void
    statusFor: (nodeId: string) => "off" | "running" | "dead"
  } = $props()

  const isFolder = $derived(node.kind === "folder")
  const terminals = $derived(node.kind === "workspace" ? node.rows * node.cols : 0)
  const childCount = $derived(node.kind === "folder" ? countWorkspaces(node.children) : 0)

  function countWorkspaces(nodes: Node[]): number {
    return nodes.reduce(
      (n, c) => n + (c.kind === "folder" ? countWorkspaces(c.children) : 1),
      0
    )
  }
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
  style="padding-left:{8 + (depth - 1) * 14}px"
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
  onpointerdown={(e) => onpointerdownitem(node, e)}
>
  {#if node.kind === "folder"}
    <svg class="chevron" class:open={node.expanded} width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><polyline points="6 9 12 15 18 9"></polyline></svg>
    <svg class="folder" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round" aria-hidden="true"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"></path></svg>
  {:else}
    <span class="dot {status}" title={t(`status.${status}`)}></span>
  {/if}
  <span class="name">{node.name}</span>
  <span class="badge">{isFolder ? childCount : terminals}</span>
</div>

{#if node.kind === "folder" && node.expanded}
  {#each node.children as child (child.id)}
    <Self node={child} depth={depth + 1} {activeId} {dragId} {dropHint}
          {onselect} {ontoggle} {oncontext}
          {onpointerdownitem} {statusFor} />
  {/each}
{/if}

<style>
  .tree-item {
    display: flex;
    align-items: center;
    gap: 8px;
    /* min-height rather than height: the row is border-box, so a fixed height
       would swallow the vertical padding instead of letting the row breathe. */
    min-height: 34px;
    padding-block: 5px;
    padding-right: 8px;
    border-radius: 7px;
    color: var(--text-2);
    font-size: calc(12.5px * var(--font-scale, 1));
    font-weight: 500;
    cursor: pointer;
    user-select: none;
    touch-action: none;
  }
  .tree-item:hover {
    background: color-mix(in srgb, var(--text-1) 6%, transparent);
    color: var(--text-1);
  }
  .tree-item.active {
    background: var(--bg-elevated);
    color: var(--text-1);
    box-shadow: inset 2px 0 0 var(--accent);
  }
  .chevron {
    flex-shrink: 0;
    color: var(--text-3);
    transform: rotate(-90deg);
    transition: transform 120ms ease;
  }
  .chevron.open {
    transform: none;
  }
  .folder {
    flex-shrink: 0;
    color: var(--text-2);
  }
  .name {
    flex: 1;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .dot {
    flex-shrink: 0;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: transparent;
    border: 1.5px solid var(--text-3);
  }
  .dot.running {
    background: var(--ok);
    border-color: var(--ok);
  }
  .dot.dead {
    background: var(--err);
    border-color: var(--err);
  }
  .badge {
    flex-shrink: 0;
    color: var(--text-3);
    font-family: var(--font-mono);
    font-size: calc(10.5px * var(--font-scale, 1));
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
