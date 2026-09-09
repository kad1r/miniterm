<script lang="ts">
  import { app, commit, notify } from "../store/app.svelte"
  import { canDrop, findNode, insert, move, remove, rename, setExpanded } from "../store/tree"
  import { zoneFor, targetFor, type DropZone } from "../store/dnd"
  import { deletePrompt, newFolder, readCollapsed, writeCollapsed } from "../store/sidebar"
  import type { Node } from "../store/types"
  import TreeItem from "./TreeItem.svelte"
  import ContextMenu from "./ContextMenu.svelte"
  import { closeSubtree, statusOf, sessions } from "../store/sessions.svelte"
  import { locale, t } from "../i18n/locale.svelte"

  let { onnew }: { onnew: () => void } = $props()

  let menu = $state<{ node: Node; x: number; y: number } | null>(null)

  // Read once at construction. localStorage is unavailable in the node test
  // environment, hence the guard rather than a bare reference.
  let collapsed = $state(
    typeof localStorage === "undefined" ? false : readCollapsed(localStorage)
  )

  // Collapsed rail shows every workspace regardless of folder expansion —
  // a collapsed folder must not hide its workspaces when the tree is gone.
  const workspaceRail = $derived(collectWorkspaces(app.config.tree))

  function collectWorkspaces(nodes: Node[]): Node[] {
    return nodes.flatMap((n) => (n.kind === "folder" ? collectWorkspaces(n.children) : [n]))
  }

  function toggleCollapsed() {
    collapsed = !collapsed
    if (typeof localStorage !== "undefined") writeCollapsed(localStorage, collapsed)
  }

  /**
   * The grid must re-measure once the sidebar has finished moving, not on every
   * animation frame. TerminalGrid already debounces window resize into exactly
   * one fit() + syncSize(), so firing that event when the transition ends
   * reuses the existing contract instead of adding a second resize path.
   *
   * Guarded on propertyName: the transition also runs on child opacity, and
   * every one of those would otherwise trigger a PTY resize.
   */
  function onTransitionEnd(e: TransitionEvent) {
    if (e.propertyName === "width") window.dispatchEvent(new Event("resize"))
  }

  function select(node: Node) {
    app.activeWorkspaceId = node.id
    app.view = "workspace"
  }

  function toggle(node: Node) {
    if (node.kind !== "folder") return
    commit((c) => ({ ...c, tree: setExpanded(c.tree, node.id, !node.expanded) }))
  }

  function renameNode(node: Node) {
    const name = prompt(t("sidebar.renamePrompt"), node.name)?.trim()
    if (!name) return
    commit((c) => ({ ...c, tree: rename(c.tree, node.id, name) }))
  }

  function liveCount(id: string): number {
    const slot = sessions.byWorkspace[id]
    if (!slot) return 0
    return slot.ids.filter((x) => x !== null).length
  }

  async function deleteNode(node: Node) {
    if (!confirm(deletePrompt(node, liveCount, locale.current))) return
    await closeSubtree(node)
    commit((c) => ({ ...c, tree: remove(c.tree, node.id).tree }))
    if (app.activeWorkspaceId && !findNode(app.config.tree, app.activeWorkspaceId)) {
      app.activeWorkspaceId = null
    }
    notify(t("sidebar.deleted", { name: node.name }))
  }

  function addFolder() {
    const f = newFolder(t("sidebar.newFolderName"))
    commit((c) => ({ ...c, tree: insert(c.tree, f, { type: "rootEnd" }) }))
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key !== "F2" || !app.activeWorkspaceId) return
    // Ignore if focus is inside a terminal pane — xterm must handle F2 itself.
    if ((e.target as Element | null)?.closest(".pane")) return
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
      notify(t("sidebar.dropRejected"), "error")
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

<aside class="sidebar" class:collapsed ontransitionend={onTransitionEnd}>
  <header>
    <button
      class="icon"
      title={collapsed ? t("sidebar.expand") : t("sidebar.collapse")}
      aria-label={collapsed ? t("sidebar.expand") : t("sidebar.collapse")}
      aria-expanded={!collapsed}
      onclick={toggleCollapsed}
    >
      {collapsed ? "»" : "«"}
    </button>
    {#if !collapsed}
      <span class="title">{t("sidebar.title")}</span>
      <button class="icon" title={t("sidebar.newFolder")} onclick={addFolder}>🗀</button>
      <button class="icon" title={t("sidebar.newWorkspace")} onclick={onnew}>+</button>
    {/if}
  </header>

  {#if collapsed}
    <!-- Icon rail: workspaces only, flattened. Folders carry no session state,
         so nesting has nothing to show at this width. -->
    <div class="rail">
      {#each workspaceRail as w (w.id)}
        <button
          class="rail-item"
          class:on={w.id === app.activeWorkspaceId}
          title={w.name}
          aria-label={w.name}
          onclick={() => select(w)}
        >
          <span class="rail-initial">{w.name.slice(0, 1).toUpperCase()}</span>
          <span class="dot {statusOf(w.id)}"></span>
        </button>
      {/each}
    </div>
  {:else}
  <div
    class="tree"
    role="tree"
    aria-label={t("sidebar.title")}
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
        statusFor={statusOf}
      />
    {/each}
    {#if app.config.tree.length === 0}
      <p class="empty">{t("sidebar.emptyTitle")}<br />{t("sidebar.emptyHint")}</p>
    {/if}
  </div>
  {/if}

  <footer>
    <button
      class="settings"
      class:on={app.view === "settings"}
      title={t("sidebar.settings")}
      aria-label={t("sidebar.settings")}
      onclick={() => (app.view = app.view === "settings" ? "workspace" : "settings")}
    >
      {collapsed ? "⚙" : `⚙ ${t("sidebar.settings")}`}
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
      { label: t("sidebar.rename"), action: () => renameNode(target) },
      { label: t("sidebar.delete"), action: () => void deleteNode(target), danger: true },
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
    transition: width 160ms ease, min-width 160ms ease;
  }
  .sidebar.collapsed {
    width: 56px;
    min-width: 56px;
  }
  @media (prefers-reduced-motion: reduce) {
    .sidebar {
      transition: none;
    }
  }
  .rail {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 4px;
    padding: 6px;
    overflow-y: auto;
  }
  .rail-item {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    height: 32px;
    border: 1px solid transparent;
    border-radius: 6px;
    background: var(--bg);
    color: var(--text-dim);
    font: inherit;
    cursor: pointer;
  }
  .rail-item:hover {
    color: var(--text);
  }
  .rail-item.on {
    border-color: var(--accent);
    color: var(--text);
  }
  .rail-initial {
    font-size: calc(13px * var(--font-scale, 1));
    font-weight: 600;
  }
  .dot {
    position: absolute;
    right: 4px;
    bottom: 4px;
    width: 6px;
    height: 6px;
    border-radius: 50%;
  }
  .dot.running {
    background: var(--ok);
  }
  .dot.dead {
    background: var(--err);
  }
  .dot.off {
    background: var(--text-dim);
    opacity: 0.4;
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
    font-size: calc(11px * var(--font-scale, 1));
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
    font-size: calc(15px * var(--font-scale, 1));
    line-height: 1;
    cursor: pointer;
  }
  .icon:hover {
    background: color-mix(in srgb, var(--text) 10%, transparent);
    color: var(--text);
  }
  /* With the title and add-buttons gone, centre what remains. */
  .sidebar.collapsed header {
    justify-content: center;
    padding: 0 6px;
  }
  .sidebar.collapsed .settings {
    text-align: center;
  }
  .tree {
    flex: 1;
    overflow-y: auto;
    padding: 6px 6px 12px;
  }
  .empty {
    margin: 24px 8px;
    color: var(--text-dim);
    font-size: calc(12px * var(--font-scale, 1));
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
    font-size: calc(13px * var(--font-scale, 1));
    text-align: left;
    cursor: pointer;
  }
  .settings:hover,
  .settings.on {
    background: color-mix(in srgb, var(--text) 10%, transparent);
    color: var(--text);
  }
</style>
