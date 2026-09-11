<script lang="ts">
  import { onMount, tick, untrack } from "svelte"
  import type { Workspace } from "../store/types"
  import { cells, dividerCount, templateWithDividers } from "../store/grid"
  import { MIN_CELL_PX, resizeFractions } from "../store/layout"
  import { isClosePaneChord } from "../term/pane"
  import { t } from "../i18n/locale.svelte"
  import TerminalPane from "./TerminalPane.svelte"

  let { workspace, active, sessionIds, exitCodes, focusedIndex, onrestart, onsizes, onclose, onfocuspane }: {
    workspace: Workspace
    active: boolean
    sessionIds: (number | null)[]
    exitCodes: (number | null | undefined)[]
    focusedIndex: number
    onrestart: (index: number) => void
    onsizes: (patch: { rowSizes?: number[]; colSizes?: number[] }) => void
    onclose: (index: number) => void
    onfocuspane: (index: number) => void
  } = $props()

  let container: HTMLDivElement
  let panes = $state<(TerminalPane | null)[]>([])

  // Sürükleme sırasındaki geçici boyutlar; bırakınca onsizes ile kalıcılaşır.
  let liveRows = $state<number[] | null>(null)
  let liveCols = $state<number[] | null>(null)

  const rowSizes = $derived(liveRows ?? workspace.rowSizes)
  const colSizes = $derived(liveCols ?? workspace.colSizes)
  const grid = $derived(cells(workspace.rows, workspace.cols))
  // The last terminal has no close button: an empty workspace would render as a
  // blank pane with no way back, and removing the workspace is a sidebar action.
  const closable = $derived(grid.length > 1)

  /** Ctrl+Shift+W, caught as it bubbles out of the focused pane. TerminalPane
   *  hands the chord back untouched, so `e.target` is still xterm's textarea and
   *  the enclosing `.cell` names the pane to close. Bound imperatively in onMount:
   *  this is a delegated shortcut, and the grid is not an interactive element. */
  function cellIndexOf(target: EventTarget | null): number | null {
    const cell = (target as HTMLElement | null)?.closest<HTMLElement>(".cell")
    if (!cell) return null
    const index = Number(cell.dataset.index)
    return Number.isInteger(index) ? index : null
  }

  function onGridKeydown(e: KeyboardEvent) {
    if (!closable || !isClosePaneChord(e)) return
    const index = cellIndexOf(e.target)
    if (index === null) return
    e.preventDefault()
    onclose(index)
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

<div
  class="grid"
  bind:this={container}
  style="grid-template-columns:{templateWithDividers(colSizes)};
         grid-template-rows:{templateWithDividers(rowSizes)}"
>
  {#each grid as cell (cell.index)}
    <div
      class="cell"
      data-index={cell.index}
      style="grid-column:{cell.col * 2 + 1}; grid-row:{cell.row * 2 + 1}"
    >
      <TerminalPane
        bind:this={panes[cell.index]}
        sessionId={sessionIds[cell.index] ?? null}
        exitCode={exitCodes[cell.index]}
        onrestart={() => onrestart(cell.index)}
      />
      {#if closable}
        <button
          class="close"
          type="button"
          title={t("grid.closePane", { n: cell.index + 1 })}
          aria-label={t("grid.closePane", { n: cell.index + 1 })}
          onclick={() => onclose(cell.index)}
        >
          <svg viewBox="0 0 12 12" aria-hidden="true">
            <path d="M3 3l6 6M9 3l-6 6" />
          </svg>
        </button>
      {/if}
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

<style>
  .grid {
    display: grid;
    width: 100%;
    height: 100%;
    background: var(--bg);
  }
  .cell {
    position: relative;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: 4px;
  }
  /* Kept out of the way until the pane is pointed at or focused, so six grids
     worth of buttons do not compete with the text. */
  .close {
    position: absolute;
    top: 4px;
    right: 4px;
    z-index: 1;
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    padding: 0;
    border: 0;
    border-radius: 4px;
    background: color-mix(in srgb, var(--bg-raised) 85%, transparent);
    color: var(--text-dim);
    opacity: 0;
    cursor: pointer;
  }
  .cell:hover .close,
  .cell:focus-within .close,
  .close:focus-visible {
    opacity: 1;
  }
  .close:hover,
  .close:focus-visible {
    background: var(--err);
    color: #fff;
    outline: none;
  }
  .close svg {
    width: 12px;
    height: 12px;
    stroke: currentColor;
    stroke-width: 1.6;
    stroke-linecap: round;
    fill: none;
  }
  /* Which terminal takes the keystrokes. :focus-within needs no state of its
     own — xterm's textarea lives inside the cell, so the browser's own notion
     of focus is already the answer. The ring is a box-shadow rather than a
     thicker border so the cell never changes size and the panes never reflow. */
  .cell:focus-within {
    border-color: var(--claude);
    box-shadow: 0 0 0 1px var(--claude);
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
