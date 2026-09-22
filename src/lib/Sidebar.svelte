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
  import { appVersion } from "../ipc"
  import { theme, toggleTheme } from "../store/theme.svelte"

  let { onnew, onaddterminal, onchangelayout }: {
    onnew: () => void
    onaddterminal: (workspaceId: string) => void
    onchangelayout: (workspaceId: string) => void
  } = $props()

  let menu = $state<{ node: Node; x: number; y: number } | null>(null)

  // The running build's version, shown in the footer. Straight from the bundle
  // metadata like the About tab, so it never drifts from what is installed.
  let version = $state("")
  appVersion().then((v) => (version = v)).catch(() => {})

  // Free-text filter over workspace/folder names. Empty query keeps the real
  // tree (and its expansion state); a live query returns a pruned copy with
  // every surviving folder force-expanded so matches are always visible.
  let query = $state("")
  const displayTree = $derived(
    query.trim() ? filterTree(app.config.tree, query.trim().toLowerCase()) : app.config.tree
  )

  function filterTree(nodes: Node[], q: string): Node[] {
    const out: Node[] = []
    for (const n of nodes) {
      if (n.kind === "folder") {
        if (n.name.toLowerCase().includes(q)) {
          out.push({ ...n, expanded: true })
        } else {
          const kids = filterTree(n.children, q)
          if (kids.length) out.push({ ...n, children: kids, expanded: true })
        }
      } else if (n.name.toLowerCase().includes(q)) {
        out.push(n)
      }
    }
    return out
  }

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

  // Reorder runs on pointer events, not HTML5 drag-and-drop: Tauri's OS-level
  // drag-drop handler (needed for file drops onto terminals) intercepts every
  // native drag over the WebView on Windows, so `dragstart`/`drop` never fire
  // inside the app. Pointer events are untouched by it.
  let dragId = $state<string | null>(null)
  let dropHint = $state<{ nodeId: string; zone: DropZone; ok: boolean } | null>(null)
  let dropRoot = $state(false)
  // Where the press began; the drag only starts once the pointer moves past a
  // small threshold, so a plain click still selects/toggles the row.
  let pending: { id: string; x: number; y: number } | null = null
  const DRAG_THRESHOLD_PX = 5

  function pointerDownItem(node: Node, e: PointerEvent) {
    if (e.button !== 0) return
    pending = { id: node.id, x: e.clientX, y: e.clientY }
  }

  function onPointerMove(e: PointerEvent) {
    if (!pending && !dragId) return
    if (pending && !dragId) {
      if (Math.hypot(e.clientX - pending.x, e.clientY - pending.y) < DRAG_THRESHOLD_PX) return
      dragId = pending.id
      dropHint = null
      dropRoot = false
    }
    if (!dragId) return
    const el = document.elementFromPoint(e.clientX, e.clientY) as HTMLElement | null
    const row = el?.closest<HTMLElement>(".tree-item")
    if (row?.dataset.nodeId) {
      const box = row.getBoundingClientRect()
      const zone = zoneFor(e.clientY - box.top, box.height, row.dataset.kind === "folder")
      const nodeId = row.dataset.nodeId
      const ok = nodeId !== dragId && canDrop(app.config.tree, dragId, targetFor(zone, nodeId))
      dropHint = { nodeId, zone, ok }
      dropRoot = false
    } else {
      // Over the tree's blank area — drop lands at the root end.
      dropHint = null
      dropRoot = el?.closest(".tree") != null
    }
  }

  function onPointerUp() {
    const hint = dropHint
    const id = dragId
    const toRoot = dropRoot
    const wasDragging = dragId !== null
    pending = null
    dragId = null
    dropHint = null
    dropRoot = false
    if (!wasDragging || !id) return
    // A completed drag must not also fire the row's click (select/toggle).
    window.addEventListener("click", killClick, { capture: true, once: true })
    if (hint) {
      if (!hint.ok) {
        notify(t("sidebar.dropRejected"), "error")
        return
      }
      commit((c) => ({ ...c, tree: move(c.tree, id, targetFor(hint.zone, hint.nodeId)) }))
    } else if (toRoot) {
      commit((c) => ({ ...c, tree: move(c.tree, id, { type: "rootEnd" }) }))
    }
  }

  function killClick(e: MouseEvent) {
    e.stopPropagation()
    e.preventDefault()
  }
</script>

<svelte:window onkeydown={onKeydown} onpointermove={onPointerMove} onpointerup={onPointerUp} onpointercancel={onPointerUp} />

<aside class="sidebar" class:collapsed ontransitionend={onTransitionEnd}>
  {#if collapsed}
    <header class="rail-head">
      <button
        class="icon"
        title={t("sidebar.expand")}
        aria-label={t("sidebar.expand")}
        aria-expanded="false"
        onclick={toggleCollapsed}
      >
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><line x1="5" y1="12" x2="19" y2="12"></line><line x1="13" y1="6" x2="19" y2="12"></line><line x1="13" y1="18" x2="19" y2="12"></line></svg>
      </button>
    </header>
  {:else}
    <header>
      <div class="search">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><circle cx="11" cy="11" r="7"></circle><line x1="16.5" y1="16.5" x2="21" y2="21"></line></svg>
        <input
          type="text"
          placeholder={t("sidebar.search")}
          aria-label={t("sidebar.title")}
          bind:value={query}
        />
      </div>
      <button class="icon" title={t("sidebar.newFolder")} aria-label={t("sidebar.newFolder")} onclick={addFolder}>
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round" aria-hidden="true"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"></path></svg>
      </button>
      <button class="icon new" title={t("sidebar.newWorkspace")} aria-label={t("sidebar.newWorkspace")} onclick={onnew}>
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><line x1="12" y1="5" x2="12" y2="19"></line><line x1="5" y1="12" x2="19" y2="12"></line></svg>
      </button>
    </header>
  {/if}

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
    class:root-drop={dropRoot}
    role="tree"
    aria-label={t("sidebar.title")}
    tabindex="0"
  >
    <div class="section">{t("sidebar.title")}</div>
    {#each displayTree as node (node.id)}
      <TreeItem
        {node}
        depth={1}
        activeId={app.activeWorkspaceId}
        {dragId}
        {dropHint}
        onselect={select}
        ontoggle={toggle}
        oncontext={(n, x, y) => (menu = { node: n, x, y })}
        onpointerdownitem={pointerDownItem}
        statusFor={statusOf}
      />
    {/each}
    {#if displayTree.length === 0}
      <p class="empty">
        {#if query.trim()}
          {t("sidebar.emptyTitle")}
        {:else}
          {t("sidebar.emptyTitle")}<br />{t("sidebar.emptyHint")}
        {/if}
      </p>
    {/if}
  </div>
  {/if}

  <footer>
    <button
      class="icon"
      title={collapsed ? t("sidebar.expand") : t("sidebar.collapse")}
      aria-label={collapsed ? t("sidebar.expand") : t("sidebar.collapse")}
      aria-expanded={!collapsed}
      onclick={toggleCollapsed}
    >
      {#if collapsed}
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><line x1="5" y1="12" x2="19" y2="12"></line><line x1="13" y1="6" x2="19" y2="12"></line><line x1="13" y1="18" x2="19" y2="12"></line></svg>
      {:else}
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round" aria-hidden="true"><rect x="3" y="4" width="18" height="16" rx="2.5"></rect><line x1="10" y1="4" x2="10" y2="20"></line></svg>
      {/if}
    </button>
    <button
      class="icon"
      title={theme.current === "dark" ? t("header.toLight") : t("header.toDark")}
      aria-label={theme.current === "dark" ? t("header.toLight") : t("header.toDark")}
      onclick={() => toggleTheme()}
    >
      {#if theme.current === "dark"}
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="4"></circle><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"></path></svg>
      {:else}
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z"></path></svg>
      {/if}
    </button>
    <button
      class="icon"
      class:on={app.view === "settings"}
      title={t("sidebar.settings")}
      aria-label={t("sidebar.settings")}
      aria-pressed={app.view === "settings"}
      onclick={() => (app.view = app.view === "settings" ? "workspace" : "settings")}
    >
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><line x1="4" y1="7" x2="20" y2="7"></line><line x1="4" y1="17" x2="20" y2="17"></line><circle cx="10" cy="7" r="2.2"></circle><circle cx="16" cy="17" r="2.2"></circle></svg>
    </button>
    {#if !collapsed}
      <div class="footer-spacer"></div>
      <span class="version">v{version}</span>
    {/if}
  </footer>
</aside>

{#if menu}
  {@const target = menu.node}
  <ContextMenu
    x={menu.x}
    y={menu.y}
    onclose={() => (menu = null)}
    items={[
      ...(target.kind === "workspace"
        ? [
            { label: t("sidebar.addTerminal"), action: () => onaddterminal(target.id) },
            { label: t("sidebar.changeLayout"), action: () => onchangelayout(target.id) },
          ]
        : []),
      { label: t("sidebar.rename"), action: () => renameNode(target) },
      { label: t("sidebar.delete"), action: () => void deleteNode(target), danger: true },
    ]}
  />
{/if}

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    width: 264px;
    min-width: 264px;
    height: 100%;
    background: var(--bg-sidebar);
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
    height: 34px;
    border: 1px solid transparent;
    border-radius: 8px;
    background: var(--bg-elevated);
    color: var(--text-2);
    font: inherit;
    cursor: pointer;
  }
  .rail-item:hover {
    color: var(--text-1);
  }
  .rail-item.on {
    border-color: var(--accent);
    color: var(--text-1);
  }
  .rail-initial {
    font-size: calc(13px * var(--font-scale, 1));
    font-weight: 600;
  }
  .rail-item .dot {
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
    background: var(--text-3);
    opacity: 0.5;
  }
  header {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 56px;
    padding: 0 12px;
  }
  header.rail-head {
    justify-content: center;
    padding: 0 6px;
  }
  .search {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    height: 34px;
    padding: 0 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-elevated);
    color: var(--text-3);
  }
  .search:focus-within {
    border-color: var(--border-strong);
  }
  .search input {
    flex: 1;
    min-width: 0;
    border: 0;
    background: none;
    color: var(--text-1);
    font: inherit;
    font-size: calc(12.5px * var(--font-scale, 1));
    outline: none;
  }
  .search input::placeholder {
    color: var(--text-3);
  }
  .icon {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    width: 34px;
    height: 34px;
    border: 1px solid transparent;
    border-radius: var(--radius);
    background: none;
    color: var(--text-2);
    cursor: pointer;
  }
  .icon:hover {
    background: color-mix(in srgb, var(--text-1) 8%, transparent);
    color: var(--text-1);
  }
  .icon.new {
    border-color: color-mix(in srgb, var(--accent) 32%, transparent);
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    color: var(--accent);
  }
  .icon.new:hover {
    background: color-mix(in srgb, var(--accent) 20%, transparent);
    color: var(--accent);
  }
  .tree {
    flex: 1;
    overflow-y: auto;
    padding: 0 8px 12px;
  }
  .tree.root-drop {
    box-shadow: inset 0 -2px 0 var(--accent);
  }
  .section {
    padding: 8px 8px 6px;
    color: var(--text-3);
    font-size: calc(10.5px * var(--font-scale, 1));
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
  .empty {
    margin: 24px 8px;
    color: var(--text-2);
    font-size: calc(12px * var(--font-scale, 1));
    line-height: 1.6;
    text-align: center;
  }
  footer {
    display: flex;
    align-items: center;
    gap: 2px;
    height: 44px;
    padding: 0 8px;
    border-top: 1px solid var(--border);
  }
  .sidebar.collapsed footer {
    flex-direction: column;
    justify-content: center;
    gap: 4px;
    height: auto;
    padding: 8px 6px;
  }
  footer .icon {
    width: 30px;
    height: 30px;
  }
  footer .icon.on {
    background: var(--bg-elevated);
    color: var(--accent);
  }
  .footer-spacer {
    flex: 1 1 auto;
  }
  .version {
    color: var(--text-3);
    font-family: var(--font-mono);
    font-size: calc(11px * var(--font-scale, 1));
  }
</style>
