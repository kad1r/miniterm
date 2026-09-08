<script lang="ts">
  import { app, commit, notify } from "../store/app.svelte"
  import { canDrop, findNode, insert, move, remove, rename, setExpanded } from "../store/tree"
  import { zoneFor, targetFor, type DropZone } from "../store/dnd"
  import { deletePrompt, newFolder } from "../store/sidebar"
  import type { Node } from "../store/types"
  import TreeItem from "./TreeItem.svelte"
  import ContextMenu from "./ContextMenu.svelte"

  let { onnew }: { onnew: () => void } = $props()

  let menu = $state<{ node: Node; x: number; y: number } | null>(null)

  function select(node: Node) {
    app.activeWorkspaceId = node.id
    app.view = "workspace"
  }

  function toggle(node: Node) {
    if (node.kind !== "folder") return
    commit((c) => ({ ...c, tree: setExpanded(c.tree, node.id, !node.expanded) }))
  }

  function renameNode(node: Node) {
    const name = prompt("Yeni ad", node.name)?.trim()
    if (!name) return
    commit((c) => ({ ...c, tree: rename(c.tree, node.id, name) }))
  }

  function deleteNode(node: Node) {
    if (!confirm(deletePrompt(node))) return
    commit((c) => ({ ...c, tree: remove(c.tree, node.id).tree }))
    if (app.activeWorkspaceId && !findNode(app.config.tree, app.activeWorkspaceId)) {
      app.activeWorkspaceId = null
    }
    notify(`"${node.name}" silindi`)
  }

  function addFolder() {
    const f = newFolder()
    commit((c) => ({ ...c, tree: insert(c.tree, f, { type: "rootEnd" }) }))
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key !== "F2" || !app.activeWorkspaceId) return
    const node = findNode(app.config.tree, app.activeWorkspaceId)
    if (node) renameNode(node)
  }

  let dragId = $state<string | null>(null)
  let dropHint = $state<{ nodeId: string; zone: DropZone; ok: boolean } | null>(null)

  function dragStart(node: Node) {
    dragId = node.id
    dropHint = null
  }

  function dragOver(node: Node, offsetY: number, height: number) {
    if (!dragId) return
    const zone = zoneFor(offsetY, height, node.kind === "folder")
    const ok = node.id !== dragId && canDrop(app.config.tree, dragId, targetFor(zone, node.id))
    dropHint = { nodeId: node.id, zone, ok }
  }

  function drop() {
    const hint = dropHint
    const id = dragId
    dragEnd()
    if (!id || !hint) return
    if (!hint.ok) {
      notify("Buraya taşınamaz", "error")
      return
    }
    commit((c) => ({ ...c, tree: move(c.tree, id, targetFor(hint.zone, hint.nodeId)) }))
  }

  function dragEnd() {
    dragId = null
    dropHint = null
  }

  function dropToRoot() {
    const id = dragId
    dragEnd()
    if (!id) return
    commit((c) => ({ ...c, tree: move(c.tree, id, { type: "rootEnd" }) }))
  }
</script>

<svelte:window onkeydown={onKeydown} />

<aside class="sidebar">
  <header>
    <span class="title">Workspaces</span>
    <button class="icon" title="Yeni klasör" onclick={addFolder}>🗀</button>
    <button class="icon" title="Yeni workspace" onclick={onnew}>+</button>
  </header>

  <div
    class="tree"
    role="tree"
    aria-label="Workspaces"
    tabindex="0"
    ondragover={(e) => e.preventDefault()}
    ondrop={dropToRoot}
  >
    {#each app.config.tree as node (node.id)}
      <TreeItem
        {node}
        depth={1}
        activeId={app.activeWorkspaceId}
        {dragId}
        {dropHint}
        onselect={select}
        ontoggle={toggle}
        oncontext={(n, x, y) => (menu = { node: n, x, y })}
        ondragstart={dragStart}
        ondragover={dragOver}
        ondrop={drop}
        ondragend={dragEnd}
      />
    {/each}
    {#if app.config.tree.length === 0}
      <p class="empty">Henüz workspace yok.<br />Başlamak için + düğmesine bas.</p>
    {/if}
  </div>

  <footer>
    <button
      class="settings"
      class:on={app.view === "settings"}
      onclick={() => (app.view = app.view === "settings" ? "workspace" : "settings")}
    >
      ⚙ Ayarlar
    </button>
  </footer>
</aside>

{#if menu}
  {@const target = menu.node}
  <ContextMenu
    x={menu.x}
    y={menu.y}
    onclose={() => (menu = null)}
    items={[
      { label: "Yeniden adlandır", action: () => renameNode(target) },
      { label: "Sil", action: () => deleteNode(target), danger: true },
    ]}
  />
{/if}

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    width: 240px;
    min-width: 240px;
    height: 100%;
    background: var(--bg-raised);
    border-right: 1px solid var(--border);
  }
  header {
    display: flex;
    align-items: center;
    gap: 2px;
    height: 38px;
    padding: 0 6px 0 10px;
    border-bottom: 1px solid var(--border);
  }
  .title {
    flex: 1;
    color: var(--text-dim);
    font-size: 11px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
  .icon {
    width: 24px;
    height: 24px;
    border: 0;
    border-radius: 4px;
    background: none;
    color: var(--text-dim);
    font-size: 15px;
    line-height: 1;
    cursor: pointer;
  }
  .icon:hover {
    background: color-mix(in srgb, var(--text) 10%, transparent);
    color: var(--text);
  }
  .tree {
    flex: 1;
    overflow-y: auto;
    padding: 6px 6px 12px;
  }
  .empty {
    margin: 24px 8px;
    color: var(--text-dim);
    font-size: 12px;
    line-height: 1.6;
    text-align: center;
  }
  footer {
    padding: 6px;
    border-top: 1px solid var(--border);
  }
  .settings {
    width: 100%;
    padding: 7px 10px;
    border: 0;
    border-radius: 4px;
    background: none;
    color: var(--text-dim);
    font: inherit;
    font-size: 13px;
    text-align: left;
    cursor: pointer;
  }
  .settings:hover,
  .settings.on {
    background: color-mix(in srgb, var(--text) 10%, transparent);
    color: var(--text);
  }
</style>
