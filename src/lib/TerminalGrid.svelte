<script lang="ts">
  import { onMount, tick, untrack } from "svelte"
  import type { Workspace } from "../store/types"
  import { cells, dividerCount, templateWithDividers } from "../store/grid"
  import { MIN_CELL_PX, resizeFractions } from "../store/layout"
  import {
    isClosePaneChord, isFocusNextPaneChord, isFocusPrevPaneChord, isMaximizePaneChord,
    isMinimizePaneChord,
  } from "../term/pane"
  import type { MinimizedPane } from "../store/sessions.svelte"
  import { basename } from "../store/tree"
  import { t } from "../i18n/locale.svelte"
  import TerminalPane from "./TerminalPane.svelte"

  let {
    workspace, active, sessionIds, exitCodes, focusedIndex, minimized, maximized,
    onrestart, onsizes, onclose, onfocuspane, onminimize, onmaximize, onrestore,
  }: {
    workspace: Workspace
    active: boolean
    sessionIds: (number | null)[]
    exitCodes: (number | null | undefined)[]
    focusedIndex: number
    minimized: MinimizedPane[]
    maximized: number | null
    onrestart: (index: number) => void
    onsizes: (patch: { rowSizes?: number[]; colSizes?: number[] }) => void
    onclose: (index: number) => void
    onfocuspane: (index: number) => void
    onminimize: (index: number) => void
    onmaximize: (index: number) => void
    onrestore: (stripIndex: number) => void
  } = $props()

  let container: HTMLDivElement
  let panes = $state<(TerminalPane | null)[]>([])

  // Sürükleme sırasındaki geçici boyutlar; bırakınca onsizes ile kalıcılaşır.
  let liveRows = $state<number[] | null>(null)
  let liveCols = $state<number[] | null>(null)

  const rowSizes = $derived(liveRows ?? workspace.rowSizes)
  const colSizes = $derived(liveCols ?? workspace.colSizes)
  const grid = $derived(cells(workspace.rows, workspace.cols))
  // The last terminal keeps no pane controls: an empty workspace would render as
  // a blank pane with no way back, removing the workspace is a sidebar action,
  // and there is nothing to maximize a lone pane over.
  const closable = $derived(grid.length > 1)
  // Every pane in a workspace shares its directory, so the header names it: the
  // folder for a quick read, the full path alongside for the exact location.
  const folderName = $derived(basename(workspace.path) || workspace.name)

  /** Ctrl+Shift+W/Z/M, caught as they bubble out of the focused pane. TerminalPane
   *  hands the chords back untouched, so `e.target` is still xterm's textarea and
   *  the enclosing `.cell` names the pane to act on. Bound imperatively in onMount:
   *  these are delegated shortcuts, and the grid is not an interactive element. */
  function cellIndexOf(target: EventTarget | null): number | null {
    const cell = (target as HTMLElement | null)?.closest<HTMLElement>(".cell")
    if (!cell) return null
    const index = Number(cell.dataset.index)
    return Number.isInteger(index) ? index : null
  }

  function onGridKeydown(e: KeyboardEvent) {
    if (!closable) return
    // Ctrl+Tab / Ctrl+Shift+Tab cycle the keyboard through the panes. A maximized
    // cell is the only visible one, so cycling to a covered pane makes no sense —
    // leave the chord alone there.
    if (isFocusNextPaneChord(e) || isFocusPrevPaneChord(e)) {
      if (maximized !== null) return
      const index = cellIndexOf(e.target)
      if (index === null) return
      e.preventDefault()
      const n = grid.length
      const next = isFocusNextPaneChord(e) ? (index + 1) % n : (index - 1 + n) % n
      panes[next]?.focus()
      return
    }
    const action = isClosePaneChord(e)
      ? onclose
      : isMinimizePaneChord(e)
        ? onminimize
        : isMaximizePaneChord(e)
          ? onmaximize
          : null
    if (!action) return
    const index = cellIndexOf(e.target)
    if (index === null) return
    e.preventDefault()
    action(index)
  }

  /** Whichever pane the user last put the caret in is the one this workspace
   *  returns to. focusin rather than a per-pane handler so the close button and
   *  xterm's own textarea both count as "this cell". */
  function onGridFocusIn(e: FocusEvent) {
    const index = cellIndexOf(e.target)
    if (index !== null) onfocuspane(index)
  }

  let drag: { axis: "row" | "col"; index: number; startPos: number; base: number[] } | null = null

  function startDrag(axis: "row" | "col", index: number, e: PointerEvent) {
    e.preventDefault()
    ;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
    drag = {
      axis,
      index,
      startPos: axis === "col" ? e.clientX : e.clientY,
      base: axis === "col" ? [...colSizes] : [...rowSizes],
    }
  }

  function moveDrag(e: PointerEvent) {
    if (!drag || !container) return
    const box = container.getBoundingClientRect()
    const delta = (drag.axis === "col" ? e.clientX : e.clientY) - drag.startPos
    const total = drag.axis === "col" ? box.width : box.height
    const next = resizeFractions(drag.base, drag.index, delta, total, MIN_CELL_PX)
    if (drag.axis === "col") liveCols = next
    else liveRows = next
    for (const pane of panes) pane?.fit()
  }

  function endDrag(e: PointerEvent) {
    if (!drag) return
    ;(e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId)
    const axis = drag.axis
    drag = null
    // PTY'yi yalnızca burada eşitle — sürükleme boyunca değil.
    if (axis === "col" && liveCols) onsizes({ colSizes: liveCols })
    if (axis === "row" && liveRows) onsizes({ rowSizes: liveRows })
    liveCols = null
    liveRows = null
    queueMicrotask(() => {
      for (const pane of panes) {
        pane?.fit()
        pane?.syncSize()
      }
    })
  }

  /**
   * Keyboard nudge for focusable dividers (a11y: arrow keys move the divider
   * by a fixed step so keyboard users can resize without a mouse).
   *
   * Pattern mirrors the drag path: each keydown accumulates into liveCols/liveRows
   * and calls only fit() (cheap DOM reflow). onsizes() + syncSize() fire exactly
   * once on keyup — identical to endDrag(). A 150 ms safety-net debounce ensures
   * the gesture still commits if keyup is lost (e.g. focus moves while key is held).
   */
  const KEYBOARD_STEP_PX = 20
  let keyNudgeTimer: ReturnType<typeof setTimeout> | undefined

  function applyNudge(axis: "row" | "col", index: number, direction: -1 | 1) {
    if (!container) return
    const box = container.getBoundingClientRect()
    const total = axis === "col" ? box.width : box.height
    const base = axis === "col" ? [...colSizes] : [...rowSizes]
    const next = resizeFractions(base, index, direction * KEYBOARD_STEP_PX, total, MIN_CELL_PX)
    if (axis === "col") liveCols = next
    else liveRows = next
    for (const pane of panes) pane?.fit()
    // Safety net: commit if keyup is never received (focus lost mid-gesture).
    clearTimeout(keyNudgeTimer)
    keyNudgeTimer = setTimeout(() => commitKeyNudge(), 150)
  }

  function commitKeyNudge() {
    clearTimeout(keyNudgeTimer)
    keyNudgeTimer = undefined
    if (liveCols) onsizes({ colSizes: liveCols })
    if (liveRows) onsizes({ rowSizes: liveRows })
    liveCols = null
    liveRows = null
    queueMicrotask(() => {
      for (const pane of panes) {
        pane?.fit()
        pane?.syncSize()
      }
    })
  }

  function onColDividerKeydown(index: number, e: KeyboardEvent) {
    if (e.key === "ArrowLeft") { e.preventDefault(); applyNudge("col", index, -1) }
    else if (e.key === "ArrowRight") { e.preventDefault(); applyNudge("col", index, 1) }
  }

  function onColDividerKeyup(_index: number, e: KeyboardEvent) {
    if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
      // Cancel the 150 ms safety-net so the timer cannot also fire: one gesture
      // must produce exactly one syncSize() call (performance contract).
      clearTimeout(keyNudgeTimer)
      keyNudgeTimer = undefined
      commitKeyNudge()
    }
  }

  function onRowDividerKeydown(index: number, e: KeyboardEvent) {
    if (e.key === "ArrowUp") { e.preventDefault(); applyNudge("row", index, -1) }
    else if (e.key === "ArrowDown") { e.preventDefault(); applyNudge("row", index, 1) }
  }

  function onRowDividerKeyup(_index: number, e: KeyboardEvent) {
    if (e.key === "ArrowUp" || e.key === "ArrowDown") {
      clearTimeout(keyNudgeTimer)
      keyNudgeTimer = undefined
      commitKeyNudge()
    }
  }

  // A relayout moves every divider at once. Panes whose sessionId changed re-attach
  // and sync themselves, but the ones that kept their session only get a fit() from
  // their ResizeObserver — the PTY would stay on the old grid's dimensions. Re-fit
  // the whole grid once the new tracks are in the DOM.
  $effect(() => {
    void workspace.rows
    void workspace.cols
    void tick().then(() => {
      for (const pane of panes) {
        pane?.fit()
        pane?.syncSize()
      }
    })
  })

  // Maximizing lifts one cell out of the grid flow to cover it, so exactly two
  // panes can have changed size: the one going up and the one coming back down.
  // The other panes keep their tracks untouched — no fit, no resize IPC for them.
  let lastMaximized: number | null = null
  $effect(() => {
    const now = maximized
    if (now === lastMaximized) return
    const before = lastMaximized
    lastMaximized = now
    void tick().then(() => {
      for (const index of new Set([before, now])) {
        if (index === null) continue
        panes[index]?.fit()
        panes[index]?.syncSize()
      }
      // The button that was clicked holds the keyboard, so hand it to the terminal
      // the user is now looking at.
      panes[now ?? before ?? 0]?.focus()
    })
  })

  // Pencere yeniden boyutlanınca 100 ms sonra bir kez eşitle.
  let resizeTimer: ReturnType<typeof setTimeout> | undefined
  function onWindowResize() {
    clearTimeout(resizeTimer)
    resizeTimer = setTimeout(() => {
      for (const pane of panes) {
        pane?.fit()
        pane?.syncSize()
      }
    }, 100)
  }

  // Restore the keyboard when this workspace comes back on screen, and hand it to
  // the surviving neighbour when a pane is closed (grid.length is what changes in
  // both the close and the add case). A hidden layer cannot hold focus, so this is
  // the only thing that puts the caret back where the user left it.
  $effect(() => {
    if (!active) return
    void grid.length
    const index = untrack(() => focusedIndex)
    void tick().then(() => panes[index]?.focus())
  })

  onMount(() => {
    container.addEventListener("keydown", onGridKeydown)
    container.addEventListener("focusin", onGridFocusIn)
    return () => {
      container.removeEventListener("keydown", onGridKeydown)
      container.removeEventListener("focusin", onGridFocusIn)
      // Clear both timers so a mid-gesture unmount (workspace delete or LRU eviction)
      // does not fire against disposed panes.
      clearTimeout(keyNudgeTimer)
      clearTimeout(resizeTimer)
    }
  })
</script>

<svelte:window onresize={onWindowResize} />

<div class="workspace">
<div
  class="grid"
  bind:this={container}
  style="grid-template-columns:{templateWithDividers(colSizes)};
         grid-template-rows:{templateWithDividers(rowSizes)}"
>
  {#each grid as cell (cell.index)}
    <div
      class="cell"
      class:max={maximized === cell.index}
      class:covered={maximized !== null && maximized !== cell.index}
      data-index={cell.index}
      style="grid-column:{cell.col * 2 + 1}; grid-row:{cell.row * 2 + 1}"
    >
      <div class="pane-header">
        <span class="pane-dot" class:dead={exitCodes[cell.index] !== undefined}></span>
        <div class="pane-loc" title={workspace.path}>
          <span class="pane-name">{folderName}</span>
          {#if workspace.path}<span class="pane-path">{workspace.path}</span>{/if}
        </div>
        {#if closable}
          <div class="pane-actions">
          <button
            class="pane-btn"
            type="button"
            title={t("grid.minimizePane", { n: cell.index + 1 })}
            aria-label={t("grid.minimizePane", { n: cell.index + 1 })}
            onclick={() => onminimize(cell.index)}
          >
            <svg viewBox="0 0 12 12" aria-hidden="true">
              <path d="M3 8h6" />
            </svg>
          </button>
          <button
            class="pane-btn"
            type="button"
            title={maximized === cell.index
              ? t("grid.unmaximizePane")
              : t("grid.maximizePane", { n: cell.index + 1 })}
            aria-label={maximized === cell.index
              ? t("grid.unmaximizePane")
              : t("grid.maximizePane", { n: cell.index + 1 })}
            aria-pressed={maximized === cell.index}
            onclick={() => onmaximize(cell.index)}
          >
            {#if maximized === cell.index}
              <svg viewBox="0 0 12 12" aria-hidden="true">
                <path d="M5 5h4v4H5zM3 7V3h4" />
              </svg>
            {:else}
              <svg viewBox="0 0 12 12" aria-hidden="true">
                <path d="M3 3h6v6H3z" />
              </svg>
            {/if}
          </button>
          <button
            class="pane-btn danger"
            type="button"
            title={t("grid.closePane", { n: cell.index + 1 })}
            aria-label={t("grid.closePane", { n: cell.index + 1 })}
            onclick={() => onclose(cell.index)}
          >
            <svg viewBox="0 0 12 12" aria-hidden="true">
              <path d="M3 3l6 6M9 3l-6 6" />
            </svg>
          </button>
          </div>
        {/if}
      </div>
      <div class="pane-wrap">
        <TerminalPane
          bind:this={panes[cell.index]}
          sessionId={sessionIds[cell.index] ?? null}
          exitCode={exitCodes[cell.index]}
          onrestart={() => onrestart(cell.index)}
        />
      </div>
    </div>
  {/each}

  {#each Array(dividerCount(workspace.cols)) as _, i (i)}
    <div
      class="divider col"
      style="grid-column:{i * 2 + 2}; grid-row:1 / -1"
      role="slider"
      aria-orientation="vertical"
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={Math.round(colSizes[i] * 100)}
      aria-label="Column divider {i + 1}"
      tabindex="0"
      onpointerdown={(e) => startDrag("col", i, e)}
      onpointermove={moveDrag}
      onpointerup={endDrag}
      onpointercancel={endDrag}
      onkeydown={(e) => onColDividerKeydown(i, e)}
      onkeyup={(e) => onColDividerKeyup(i, e)}
    ></div>
  {/each}

  {#each Array(dividerCount(workspace.rows)) as _, i (i)}
    <div
      class="divider row"
      style="grid-row:{i * 2 + 2}; grid-column:1 / -1"
      role="slider"
      aria-orientation="horizontal"
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={Math.round(rowSizes[i] * 100)}
      aria-label="Row divider {i + 1}"
      tabindex="0"
      onpointerdown={(e) => startDrag("row", i, e)}
      onpointermove={moveDrag}
      onpointerup={endDrag}
      onpointercancel={endDrag}
      onkeydown={(e) => onRowDividerKeydown(i, e)}
      onkeyup={(e) => onRowDividerKeyup(i, e)}
    ></div>
  {/each}
</div>

{#if minimized.length > 0}
  <div class="dock" role="group" aria-label={t("grid.minimizedStrip")}>
    {#each minimized as item, i (i)}
      <button
        class="chip"
        type="button"
        title={t("grid.restorePane", { n: i + 1 })}
        aria-label={t("grid.restorePane", { n: i + 1 })}
        onclick={() => onrestore(i)}
      >
        <span class="dot" class:dead={item.exit !== undefined}></span>
        <span>{t("grid.minimizedPane", { n: i + 1 })}</span>
        <svg viewBox="0 0 12 12" aria-hidden="true">
          <path d="M3 7l3-3 3 3" />
        </svg>
      </button>
    {/each}
  </div>
{/if}
</div>

<style>
  .workspace {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    min-height: 0;
    gap: 6px;
  }
  .grid {
    /* Positioned so a maximized cell can take `inset: 0` against the grid area
       rather than the window. */
    position: relative;
    display: grid;
    flex: 1;
    min-height: 0;
    padding: 12px;
    background: var(--bg-app);
  }
  .cell {
    position: relative;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--bg-term);
  }
  /* Shell fills whatever the header leaves. min-height:0 lets it shrink so the
     header is never pushed off-screen in a short cell. */
  .pane-wrap {
    flex: 1;
    min-height: 0;
  }
  /* Maximize deliberately does not touch the grid tracks: the other panes keep
     their size underneath, so coming back costs no reflow and no resize IPC. */
  .cell.max {
    position: absolute;
    inset: 0;
    z-index: 3;
  }
  .cell.covered {
    /* The maximized pane covers these anyway; blocking the pointer stops a click
       that lands on a sliver of border from stealing focus. */
    pointer-events: none;
  }
  /* A real header bar above the shell, not an overlay — the buttons can never
     sit on top of terminal text now, so typing no longer collides with them. */
  .pane-header {
    flex: 0 0 auto;
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 32px;
    padding: 0 6px 0 10px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-elevated);
  }
  .pane-dot {
    flex: 0 0 auto;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--ok);
  }
  .pane-dot.dead {
    background: var(--err);
  }
  .pane-loc {
    flex: 1;
    display: flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
    overflow: hidden;
  }
  .pane-name {
    flex: 0 0 auto;
    color: var(--text-1);
    font-size: calc(11.5px * var(--font-scale, 1));
    font-weight: 600;
  }
  .pane-path {
    min-width: 0;
    overflow: hidden;
    color: var(--text-3);
    font-family: var(--font-mono);
    font-size: calc(10.5px * var(--font-scale, 1));
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .pane-actions {
    flex: 0 0 auto;
    display: flex;
    gap: 2px;
  }
  .pane-btn {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--text-2);
    cursor: pointer;
  }
  .pane-btn:hover,
  .pane-btn:focus-visible {
    background: color-mix(in srgb, var(--text-1) 10%, transparent);
    color: var(--text-1);
    outline: none;
  }
  .pane-btn.danger:hover,
  .pane-btn.danger:focus-visible {
    background: var(--err);
    color: #fff;
  }
  .pane-btn[aria-pressed="true"] {
    color: var(--accent);
  }
  .pane-btn svg,
  .chip svg {
    width: 12px;
    height: 12px;
    stroke: currentColor;
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-linejoin: round;
    fill: none;
  }
  /* The strip minimized terminals wait in. Sized by its content so an empty
     strip costs no vertical space at all. */
  .dock {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    flex: 0 0 auto;
  }
  .chip {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-raised);
    color: var(--text-dim);
    font: inherit;
    font-size: calc(12px * var(--font-scale, 1));
    cursor: pointer;
  }
  .chip:hover,
  .chip:focus-visible {
    border-color: var(--claude);
    color: var(--text);
    outline: none;
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--ok);
  }
  .dot.dead {
    background: var(--err);
  }
  /* Which terminal takes the keystrokes. :focus-within needs no state of its
     own — xterm's textarea lives inside the cell, so the browser's own notion
     of focus is already the answer. The ring is a box-shadow rather than a
     thicker border so the cell never changes size and the panes never reflow. */
  .cell:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 15%, transparent);
  }
  .divider {
    z-index: 2;
    background: transparent;
  }
  .divider:hover,
  .divider:focus-visible {
    background: color-mix(in srgb, var(--accent) 45%, transparent);
    outline: none;
  }
  .divider.col {
    cursor: col-resize;
  }
  .divider.row {
    cursor: row-resize;
  }
</style>
