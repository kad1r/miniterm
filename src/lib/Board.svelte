<script lang="ts">
  import { onMount } from "svelte"
  import { commit, notify } from "../store/app.svelte"
  import {
    addCard, addColumn, addTag, allTags, boardOf, cardCount, columnColor, dueState, formatDue,
    isoDay, moveCard, nextColor, removeCard, removeColumn, renameColumn, setColumnColor,
    shiftColumn, tagColor, updateCard,
  } from "../store/board"
  import { updateWorkspace } from "../store/tree"
  import type { Board, BoardCard, BoardColumn, Workspace } from "../store/types"
  import { locale, t } from "../i18n/locale.svelte"
  import ContextMenu from "./ContextMenu.svelte"

  let { workspace }: { workspace: Workspace } = $props()

  const board = $derived(boardOf(workspace, t))

  // Read once: the board is remounted every time it is opened, and a due chip
  // changing colour at midnight while it is on screen is not worth a timer.
  const today = isoDay(new Date())

  const dueLabel = (due: string) => formatDue(due, today, locale.current)

  function dueTitle(due: string): string {
    const state = dueState(due, today)
    if (state === "overdue") return t("board.overdue")
    if (state === "today") return t("board.dueToday")
    return t("board.due")
  }

  function save(next: Board) {
    if (next === board) return
    const id = workspace.id
    commit((c) => ({ ...c, tree: updateWorkspace(c.tree, id, { board: next }) }))
  }

  // The terminal that was focused is now hidden behind the board; without this
  // it would keep receiving every keystroke typed while looking at the board.
  onMount(() => (document.activeElement as HTMLElement | null)?.blur())

  // ── Adding cards ──────────────────────────────────────────────────────────
  let addingTo = $state<string | null>(null)
  let draft = $state("")

  function startAdding(columnId: string) {
    addingTo = columnId
    draft = ""
  }

  /** Enter keeps the field open for the next card — a backlog is usually typed
   *  several items at a time. */
  function submitDraft() {
    const title = draft.trim()
    if (!title || !addingTo) return
    save(addCard(board, addingTo, {
      id: crypto.randomUUID(), title, note: "", tags: [], due: null,
    }))
    draft = ""
  }

  function onDraftKey(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault()
      submitDraft()
    } else if (e.key === "Escape") {
      e.stopPropagation()
      // Cleared first: closing removes the field, and the blur that may follow
      // would otherwise save the text the user just backed out of.
      draft = ""
      addingTo = null
    }
  }

  // ── Editing a card ────────────────────────────────────────────────────────
  let editing = $state<BoardCard | null>(null)
  let dialogEl = $state<HTMLDialogElement | null>(null)

  $effect(() => {
    if (editing && dialogEl && !dialogEl.open) dialogEl.showModal()
  })

  function saveEdit() {
    if (!editing) return
    const title = editing.title.trim()
    if (!title) return
    // A tag still sitting in the field counts: typing one and pressing Save
    // without Enter first should not silently drop it.
    const tags = addTag(editing.tags, tagDraft)
    save(updateCard(board, editing.id, {
      title, note: editing.note.trim(), tags, due: editing.due || null,
    }))
    editing = null
  }

  let tagDraft = $state("")

  function openEditor(card: BoardCard) {
    editing = { ...card, tags: [...card.tags] }
    tagDraft = ""
  }

  function commitTag() {
    if (!editing) return
    editing.tags = addTag(editing.tags, tagDraft)
    tagDraft = ""
  }

  function onTagKey(e: KeyboardEvent) {
    if (!editing) return
    if (e.key === "Enter" || e.key === ",") {
      // Enter must add the tag, not submit the whole form.
      e.preventDefault()
      commitTag()
    } else if (e.key === "Backspace" && tagDraft === "" && editing.tags.length > 0) {
      editing.tags = editing.tags.slice(0, -1)
    }
  }

  /** Tags used elsewhere on the board that this card does not have yet. */
  const suggestions = $derived.by(() => {
    const current = editing
    if (!current) return []
    const mine = new Set(current.tags.map((x) => x.toLowerCase()))
    return allTags(board).filter((x) => !mine.has(x.toLowerCase()))
  })

  function deleteEditing() {
    if (!editing) return
    save(removeCard(board, editing.id))
    editing = null
  }

  // ── Columns ───────────────────────────────────────────────────────────────
  let menu = $state<{ column: BoardColumn; x: number; y: number } | null>(null)

  function openMenu(column: BoardColumn, e: MouseEvent) {
    const box = (e.currentTarget as HTMLElement).getBoundingClientRect()
    menu = { column, x: box.left, y: box.bottom + 4 }
  }

  function newColumn() {
    const name = prompt(t("board.columnPrompt"))?.trim()
    if (!name) return
    save(addColumn(board, { id: crypto.randomUUID(), name, cards: [] }))
  }

  function rename(column: BoardColumn) {
    const name = prompt(t("board.columnPrompt"), column.name)?.trim()
    if (name) save(renameColumn(board, column.id, name))
  }

  function remove(column: BoardColumn) {
    if (board.columns.length === 1) {
      notify(t("board.lastColumn"), "error")
      return
    }
    if (column.cards.length > 0) {
      const at = board.columns.findIndex((c) => c.id === column.id)
      const heir = board.columns[at === 0 ? 1 : at - 1].name
      const question = t("board.deleteColumnConfirm", {
        name: column.name, count: column.cards.length, heir,
      })
      if (!confirm(question)) return
    }
    save(removeColumn(board, column.id))
  }

  // ── Dragging cards ────────────────────────────────────────────────────────
  // Pointer events, not HTML5 drag-and-drop, for the reason given in Sidebar:
  // Tauri's OS-level drop handler swallows native drags over the WebView.
  let dragId = $state<string | null>(null)
  let hint = $state<{ columnId: string; before: string | null } | null>(null)
  let pending: { id: string; x: number; y: number } | null = null
  const DRAG_THRESHOLD_PX = 5

  function pointerDownCard(card: BoardCard, e: PointerEvent) {
    if (e.button !== 0) return
    pending = { id: card.id, x: e.clientX, y: e.clientY }
  }

  function onPointerMove(e: PointerEvent) {
    if (!pending && !dragId) return
    if (pending && !dragId) {
      if (Math.hypot(e.clientX - pending.x, e.clientY - pending.y) < DRAG_THRESHOLD_PX) return
      dragId = pending.id
    }
    const el = document.elementFromPoint(e.clientX, e.clientY) as HTMLElement | null
    const column = el?.closest<HTMLElement>(".column")
    const columnId = column?.dataset.columnId
    if (!column || !columnId) {
      hint = null
      return
    }
    // The slot is the first card whose middle is below the pointer. Measured
    // against every card rather than the one under the pointer, so the gaps
    // between cards resolve to a slot too instead of jumping to the bottom.
    let before: string | null = null
    for (const card of column.querySelectorAll<HTMLElement>(".card")) {
      const box = card.getBoundingClientRect()
      if (e.clientY < box.top + box.height / 2) {
        before = card.dataset.cardId ?? null
        break
      }
    }
    if (hint?.columnId !== columnId || hint.before !== before) hint = { columnId, before }
  }

  function onPointerUp() {
    const id = dragId
    const target = hint
    pending = null
    dragId = null
    hint = null
    if (!id) return
    // A finished drag must not also open the card it ended on.
    window.addEventListener("click", killClick, { capture: true, once: true })
    if (target) save(moveCard(board, id, target.columnId, target.before))
  }

  function killClick(e: MouseEvent) {
    e.stopPropagation()
    e.preventDefault()
  }

  function focusOnMount(el: HTMLElement) {
    el.focus()
  }
</script>

<svelte:window onpointermove={onPointerMove} onpointerup={onPointerUp} onpointercancel={onPointerUp} />

<section class="board" class:dragging={dragId !== null} aria-label={t("board.title")}>
  <header class="bar">
    <h1>{workspace.name}</h1>
    <span class="tally">{t("board.tally", { n: cardCount(board) })}</span>
    <div class="spacer"></div>
    <button class="ghost" onclick={newColumn}>
      <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><line x1="12" y1="5" x2="12" y2="19"></line><line x1="5" y1="12" x2="19" y2="12"></line></svg>
      {t("board.addColumn")}
    </button>
  </header>

  <div class="columns">
    {#each board.columns as column, i (column.id)}
      {@const color = columnColor(column, i)}
      <div
        class="column"
        class:target={hint?.columnId === column.id}
        data-column-id={column.id}
        style="--c: var(--board-{color})"
      >
        <header>
          <button
            class="swatch"
            title={t("board.columnColor")}
            aria-label={t("board.columnColor")}
            onclick={() => save(setColumnColor(board, column.id, nextColor(color)))}
          ></button>
          <h2 title={column.name}>{column.name}</h2>
          <span class="count">{column.cards.length}</span>
          <button
            class="icon"
            title={t("board.columnMenu")}
            aria-label={t("board.columnMenu")}
            onpointerdown={(e) => e.stopPropagation()}
            onclick={(e) => openMenu(column, e)}
          >
            <svg width="15" height="15" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><circle cx="5" cy="12" r="1.7"></circle><circle cx="12" cy="12" r="1.7"></circle><circle cx="19" cy="12" r="1.7"></circle></svg>
          </button>
        </header>

        <div class="cards">
          {#each column.cards as card (card.id)}
            {#if hint?.columnId === column.id && hint.before === card.id}
              <div class="slot"></div>
            {/if}
            <button
              class="card"
              class:lifted={dragId === card.id}
              data-card-id={card.id}
              onpointerdown={(e) => pointerDownCard(card, e)}
              onclick={() => openEditor(card)}
            >
              {#if card.tags.length > 0}
                <span class="tags">
                  {#each card.tags as tag (tag)}
                    <span class="tag" style="--c: var(--board-{tagColor(tag)})">{tag}</span>
                  {/each}
                </span>
              {/if}
              <span class="card-title">{card.title}</span>
              {#if card.note}
                <span class="card-note">{card.note}</span>
              {/if}
              {#if card.due}
                <span class="due {dueState(card.due, today)}" title={dueTitle(card.due)}>
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="3.5" y="5" width="17" height="15.5" rx="2.5"></rect><path d="M3.5 10h17M8 3v4M16 3v4"></path></svg>
                  {dueLabel(card.due)}
                </span>
              {/if}
            </button>
          {/each}
          {#if hint?.columnId === column.id && hint.before === null}
            <div class="slot"></div>
          {/if}

          {#if addingTo === column.id}
            <textarea
              class="draft"
              rows="2"
              placeholder={t("board.cardPlaceholder")}
              bind:value={draft}
              onkeydown={onDraftKey}
              onblur={() => {
                submitDraft()
                addingTo = null
              }}
              use:focusOnMount
            ></textarea>
          {/if}
        </div>

        {#if addingTo !== column.id}
          <button class="add" onclick={() => startAdding(column.id)}>
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><line x1="12" y1="5" x2="12" y2="19"></line><line x1="5" y1="12" x2="19" y2="12"></line></svg>
            {t("board.addCard")}
          </button>
        {/if}
      </div>
    {/each}
  </div>
</section>

{#if menu}
  {@const column = menu.column}
  {@const at = board.columns.findIndex((c) => c.id === column.id)}
  <ContextMenu
    x={menu.x}
    y={menu.y}
    onclose={() => (menu = null)}
    items={[
      { label: t("board.rename"), action: () => rename(column) },
      ...(at > 0
        ? [{ label: t("board.moveLeft"), action: () => save(shiftColumn(board, column.id, -1)) }]
        : []),
      ...(at < board.columns.length - 1
        ? [{ label: t("board.moveRight"), action: () => save(shiftColumn(board, column.id, 1)) }]
        : []),
      { label: t("board.deleteColumn"), action: () => remove(column), danger: true },
    ]}
  />
{/if}

{#if editing}
  <dialog
    bind:this={dialogEl}
    aria-modal="true"
    aria-label={t("board.editCard")}
    oncancel={(e) => { e.preventDefault(); editing = null }}
  >
    <form class="dialog-inner" onsubmit={(e) => { e.preventDefault(); saveEdit() }}>
      <div class="fields">
        <input
          class="edit-title"
          type="text"
          aria-label={t("board.cardTitle")}
          placeholder={t("board.cardTitle")}
          bind:value={editing.title}
        />
        <textarea
          rows="7"
          aria-label={t("board.cardNote")}
          placeholder={t("board.cardNote")}
          bind:value={editing.note}
        ></textarea>

        <div class="field">
          <span class="field-label">{t("board.tags")}</span>
          <div class="tag-box">
            {#each editing.tags as tag (tag)}
              <span class="tag" style="--c: var(--board-{tagColor(tag)})">
                {tag}
                <button
                  type="button"
                  class="tag-x"
                  aria-label={t("board.removeTag", { tag })}
                  onclick={() => editing && (editing.tags = editing.tags.filter((x) => x !== tag))}
                >×</button>
              </span>
            {/each}
            <input
              class="tag-input"
              type="text"
              aria-label={t("board.tags")}
              placeholder={t("board.tagPlaceholder")}
              bind:value={tagDraft}
              onkeydown={onTagKey}
            />
          </div>
          {#if suggestions.length > 0}
            <div class="suggestions">
              {#each suggestions as tag (tag)}
                <button
                  type="button"
                  class="tag pick"
                  style="--c: var(--board-{tagColor(tag)})"
                  onclick={() => editing && (editing.tags = addTag(editing.tags, tag))}
                >+ {tag}</button>
              {/each}
            </div>
          {/if}
        </div>

        <div class="field">
          <span class="field-label">{t("board.due")}</span>
          <div class="due-row">
            <input type="date" aria-label={t("board.due")} bind:value={editing.due} />
            {#if editing.due}
              <button type="button" class="ghost" onclick={() => editing && (editing.due = null)}>
                {t("board.clearDue")}
              </button>
            {/if}
          </div>
        </div>
      </div>
      <footer>
        <button type="button" class="ghost danger" onclick={deleteEditing}>{t("board.deleteCard")}</button>
        <span class="spacer"></span>
        <button type="button" class="ghost" onclick={() => (editing = null)}>{t("settings.cancel")}</button>
        <button type="submit" class="primary" disabled={!editing.title.trim()}>{t("board.save")}</button>
      </footer>
    </form>
  </dialog>
{/if}

<style>
  .board {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    background: var(--bg-app);
    font-size: calc(13px * var(--font-scale, 1));
  }
  .board.dragging {
    cursor: grabbing;
    user-select: none;
  }
  .bar {
    flex: 0 0 auto;
    display: flex;
    align-items: baseline;
    gap: 10px;
    padding: 16px 20px 12px;
  }
  h1 {
    margin: 0;
    font-size: calc(16px * var(--font-scale, 1));
    font-weight: 600;
    color: var(--text-1);
  }
  .tally {
    color: var(--text-3);
    font-size: calc(12px * var(--font-scale, 1));
  }
  .spacer {
    flex: 1 1 auto;
  }

  .columns {
    flex: 1 1 auto;
    display: flex;
    align-items: flex-start;
    gap: 12px;
    min-height: 0;
    padding: 4px 20px 20px;
    overflow-x: auto;
  }
  .column {
    flex: 0 0 280px;
    display: flex;
    flex-direction: column;
    max-height: 100%;
    border: 1px solid var(--border);
    /* The column's own colour, as a stripe across the top. */
    border-top: 2px solid var(--c);
    border-radius: var(--radius-md);
    background:
      linear-gradient(color-mix(in srgb, var(--c) 7%, transparent), transparent 72px),
      var(--bg-sidebar);
  }
  .column.target {
    border-color: color-mix(in srgb, var(--c) 55%, var(--border));
    border-top-color: var(--c);
  }
  .swatch {
    flex: none;
    width: 10px;
    height: 10px;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: var(--c);
    cursor: pointer;
  }
  .swatch:hover {
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--c) 28%, transparent);
  }
  .swatch:focus-visible {
    outline: none;
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--c) 45%, transparent);
  }
  .column > header {
    flex: 0 0 auto;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 8px 8px 12px;
  }
  h2 {
    margin: 0;
    min-width: 0;
    overflow: hidden;
    color: color-mix(in srgb, var(--c) 55%, var(--text-1));
    font-size: calc(11.5px * var(--font-scale, 1));
    font-weight: 600;
    letter-spacing: 0.06em;
    text-overflow: ellipsis;
    text-transform: uppercase;
    white-space: nowrap;
  }
  .count {
    padding: 1px 7px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--c) 16%, transparent);
    color: var(--c);
    font-family: var(--font-mono);
    font-size: calc(10.5px * var(--font-scale, 1));
  }
  .icon {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    margin-left: auto;
    padding: 0;
    border: 0;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--text-3);
    cursor: pointer;
  }
  .icon:hover {
    background: color-mix(in srgb, var(--text-1) 8%, transparent);
    color: var(--text-1);
  }

  .cards {
    flex: 1 1 auto;
    display: flex;
    flex-direction: column;
    gap: 6px;
    /* Keeps an empty column tall enough to be an easy drop target. */
    min-height: 28px;
    padding: 2px 8px 8px;
    overflow-y: auto;
  }
  .card {
    flex: 0 0 auto;
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 9px 11px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-elevated);
    color: var(--text-1);
    font: inherit;
    text-align: left;
    cursor: pointer;
    user-select: none;
  }
  .card:hover {
    border-color: var(--border-strong);
  }
  .card:focus-visible,
  .add:focus-visible,
  .icon:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
    border-color: var(--accent);
  }
  .card.lifted {
    opacity: 0.35;
  }
  .card-title {
    overflow-wrap: anywhere;
    line-height: 1.4;
  }
  .card-note {
    display: -webkit-box;
    overflow: hidden;
    color: var(--text-3);
    font-size: calc(12px * var(--font-scale, 1));
    line-height: 1.4;
    overflow-wrap: anywhere;
    white-space: pre-line;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
  }
  .tags,
  .tag-box,
  .suggestions {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .tag {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    max-width: 100%;
    padding: 1px 7px;
    border: 1px solid color-mix(in srgb, var(--c) 30%, transparent);
    border-radius: 999px;
    background: color-mix(in srgb, var(--c) 14%, transparent);
    color: var(--c);
    font: inherit;
    font-size: calc(11px * var(--font-scale, 1));
    font-weight: 500;
    line-height: 1.5;
    overflow-wrap: anywhere;
  }
  .due {
    display: inline-flex;
    align-items: center;
    align-self: flex-start;
    gap: 5px;
    margin-top: 2px;
    padding: 1px 7px 1px 6px;
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--text-1) 7%, transparent);
    color: var(--text-2);
    font-family: var(--font-mono);
    font-size: calc(10.5px * var(--font-scale, 1));
    line-height: 1.6;
  }
  .due.soon,
  .due.today {
    background: color-mix(in srgb, var(--warn) 16%, transparent);
    color: var(--warn);
  }
  .due.overdue {
    background: color-mix(in srgb, var(--err) 16%, transparent);
    color: var(--err);
  }
  /* Where the dragged card will land. */
  .slot {
    flex: 0 0 auto;
    height: 2px;
    margin: -1px 2px;
    border-radius: 1px;
    background: var(--accent);
    pointer-events: none;
  }

  .draft,
  dialog input,
  dialog textarea {
    width: 100%;
    padding: 8px 10px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    background: var(--bg-term);
    color: var(--text-1);
    font: inherit;
    line-height: 1.4;
    resize: none;
  }
  .draft {
    flex: 0 0 auto;
  }
  .draft:focus,
  dialog input:focus,
  dialog textarea:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: var(--focus-ring);
  }

  .add {
    flex: 0 0 auto;
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0 8px 8px;
    padding: 7px 8px;
    border: 1px solid transparent;
    border-radius: var(--radius);
    background: none;
    color: var(--text-3);
    font: inherit;
    font-size: calc(12px * var(--font-scale, 1));
    cursor: pointer;
  }
  .add:hover {
    background: color-mix(in srgb, var(--text-1) 6%, transparent);
    color: var(--text-1);
  }

  .ghost,
  .primary {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 7px 14px;
    border-radius: var(--radius-sm);
    font: inherit;
    font-size: calc(12.5px * var(--font-scale, 1));
    cursor: pointer;
  }
  .ghost {
    border: 1px solid var(--border);
    background: none;
    color: var(--text-2);
  }
  .ghost:hover {
    border-color: var(--border-strong);
    color: var(--text-1);
  }
  .ghost.danger {
    color: var(--err);
  }
  .ghost.danger:hover {
    border-color: var(--err);
    color: var(--err);
  }
  .primary {
    border: 0;
    background: var(--accent);
    color: #fff;
  }
  .primary:disabled {
    opacity: 0.45;
    cursor: default;
  }

  dialog {
    width: min(520px, 92vw);
    max-width: 92vw;
    padding: 0;
    border: none;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-1);
    font-size: calc(13px * var(--font-scale, 1));
  }
  dialog::backdrop {
    background: rgb(0 0 0 / 0.5);
  }
  .dialog-inner {
    display: flex;
    flex-direction: column;
    max-height: 90vh;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--bg-elevated);
    overflow: hidden;
  }
  /* The fields scroll; the buttons below stay reachable on a short window. */
  .fields {
    flex: 1 1 auto;
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-height: 0;
    padding: 18px 18px 14px;
    overflow-y: auto;
  }
  .fields > * {
    flex: none;
  }
  .edit-title {
    font-weight: 600;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .field-label {
    color: var(--text-3);
    font-size: calc(11px * var(--font-scale, 1));
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  /* Looks like one text field holding chips and a caret. */
  .tag-box {
    align-items: center;
    padding: 6px 8px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    background: var(--bg-term);
  }
  .tag-box:focus-within {
    border-color: var(--accent);
    box-shadow: var(--focus-ring);
  }
  dialog input.tag-input {
    flex: 1 1 140px;
    width: auto;
    min-width: 0;
    padding: 2px;
    border: 0;
    background: none;
    box-shadow: none;
  }
  .tag-x {
    padding: 0 1px;
    border: 0;
    background: none;
    color: inherit;
    font: inherit;
    line-height: 1;
    cursor: pointer;
    opacity: 0.7;
  }
  .tag-x:hover {
    opacity: 1;
  }
  .tag.pick {
    border-style: dashed;
    background: none;
    cursor: pointer;
  }
  .tag.pick:hover {
    background: color-mix(in srgb, var(--c) 14%, transparent);
  }
  .due-row {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  dialog input[type="date"] {
    flex: 0 1 190px;
    width: auto;
    font-family: var(--font-mono);
  }
  dialog footer {
    flex: none;
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    padding: 12px 18px;
    border-top: 1px solid var(--border);
  }
</style>
