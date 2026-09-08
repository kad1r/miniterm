<script lang="ts">
  import type { Workspace } from "../store/types"
  import { cells, dividerCount, templateWithDividers } from "../store/grid"
  import { MIN_CELL_PX, resizeFractions } from "../store/layout"
  import TerminalPane from "./TerminalPane.svelte"

  let { workspace, sessionIds, exitCodes, onrestart, onsizes }: {
    workspace: Workspace
    sessionIds: (number | null)[]
    exitCodes: (number | null | undefined)[]
    onrestart: (index: number) => void
    onsizes: (patch: { rowSizes?: number[]; colSizes?: number[] }) => void
  } = $props()

  let container: HTMLDivElement
  let panes = $state<(TerminalPane | null)[]>([])

  // Sürükleme sırasındaki geçici boyutlar; bırakınca onsizes ile kalıcılaşır.
  let liveRows = $state<number[] | null>(null)
  let liveCols = $state<number[] | null>(null)

  const rowSizes = $derived(liveRows ?? workspace.rowSizes)
  const colSizes = $derived(liveCols ?? workspace.colSizes)
  const grid = $derived(cells(workspace.rows, workspace.cols))

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
    if (e.key === "ArrowLeft" || e.key === "ArrowRight") commitKeyNudge()
  }

  function onRowDividerKeydown(index: number, e: KeyboardEvent) {
    if (e.key === "ArrowUp") { e.preventDefault(); applyNudge("row", index, -1) }
    else if (e.key === "ArrowDown") { e.preventDefault(); applyNudge("row", index, 1) }
  }

  function onRowDividerKeyup(_index: number, e: KeyboardEvent) {
    if (e.key === "ArrowUp" || e.key === "ArrowDown") commitKeyNudge()
  }

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
</script>

<svelte:window onresize={onWindowResize} />

<div
  class="grid"
  bind:this={container}
  style="grid-template-columns:{templateWithDividers(colSizes)};
         grid-template-rows:{templateWithDividers(rowSizes)}"
>
  {#each grid as cell (cell.index)}
    <div class="cell" style="grid-column:{cell.col * 2 + 1}; grid-row:{cell.row * 2 + 1}">
      <TerminalPane
        bind:this={panes[cell.index]}
        sessionId={sessionIds[cell.index] ?? null}
        exitCode={exitCodes[cell.index]}
        onrestart={() => onrestart(cell.index)}
      />
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
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: 4px;
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
