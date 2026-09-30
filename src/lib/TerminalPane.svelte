<script lang="ts">
  import { onMount } from "svelte"
  import { Terminal } from "@xterm/xterm"
  import { FitAddon } from "@xterm/addon-fit"
  import { WebglAddon } from "@xterm/addon-webgl"
  import "@xterm/xterm/css/xterm.css"
  import {
    attachSession, detachSession, getBuffer, onFileDrop, resizeSession, writeSession,
    type FileDrop,
  } from "../ipc"
  import { clipboardAction } from "../term/clipboard"
  import { win32KeySequence } from "../term/win32"
  import { exitNotice } from "../term/exit"
  import {
    isClosePaneChord, isFocusNextPaneChord, isFocusPrevPaneChord, isMaximizePaneChord,
    isMinimizePaneChord,
  } from "../term/pane"
  import { isAppShortcut } from "../term/shortcuts"
  import { dropText, type ShellKind } from "../term/paths"
  import { locale } from "../i18n/locale.svelte"
  import { TERM_FONT, TERM_FONT_SIZE, TERM_THEMES } from "../term/theme"
  import { fontAction, termFontSize } from "../store/font"
  import { fontScale } from "../store/font.svelte"
  import { theme } from "../store/theme.svelte"

  let {
    sessionId, exitCode = undefined, active = true, shell = "posix", onrestart, onsnapshot,
  }: {
    sessionId: number | null
    exitCode?: number | null | undefined
    /** False while the pane's workspace is a hidden layer. The WebGL renderer
     *  is only held while visible: WebView2 keeps ~16 live WebGL contexts, and
     *  one per mounted pane across every open workspace runs past that. */
    active?: boolean
    /** The pane's shell family, so a dropped path is quoted the way that
     *  shell reads it. */
    shell?: ShellKind
    onrestart?: () => void
    /** Plain-text tail of the screen, delivered at most every SNAPSHOT_MS and
     *  only after new output — feeds the agent's shared `<Name>.log`. */
    onsnapshot?: (lines: string[]) => void
  } = $props()

  const SNAPSHOT_MS = 2000
  const SNAPSHOT_LINES = 300
  // Set by every write, cleared by the snapshot: an idle pane costs nothing.
  let dirty = false

  function snapshot() {
    if (!dirty || !term || !onsnapshot) return
    dirty = false
    const buf = term.buffer.active
    const lines: string[] = []
    for (let i = Math.max(0, buf.length - SNAPSHOT_LINES); i < buf.length; i++) {
      lines.push(buf.getLine(i)?.translateToString(true) ?? "")
    }
    onsnapshot(lines)
  }

  let host: HTMLDivElement
  let term: Terminal | null = null
  let fitAddon: FitAddon | null = null
  let attached: number | null = null
  let noticeShown = false
  const encoder = new TextEncoder()

  /** Updates the DOM geometry only; never touches the PTY. Safe mid-drag. */
  export function fit() {
    if (!term || !fitAddon || !host?.isConnected || host.clientWidth < 2) return
    fitAddon.fit()
  }

  /** Matches the PTY size to the last fit(). Called once when a drag ends. */
  export function syncSize() {
    if (!term || attached === null) return
    void resizeSession(attached, term.cols, term.rows)
  }

  export function focus() {
    term?.focus()
  }

  /**
   * Drop a file on the terminal and get its quoted path at the cursor, like
   * every native terminal. Tauri delivers the drop to the whole window, so each
   * pane hit-tests the point itself: elementFromPoint skips `visibility:hidden`
   * and `pointer-events:none` nodes, which is what keeps a background workspace
   * layer or an open dialog from swallowing the drop.
   */
  function handleDrop(drop: FileDrop) {
    if (!term || attached === null || exitCode !== undefined) return
    const hit = document.elementFromPoint(drop.x, drop.y)
    if (!hit || !host.contains(hit)) return
    const text = dropText(drop.paths, shell)
    if (!text) return
    term.focus()
    void writeSession(attached, encoder.encode(text))
  }

  onMount(() => {
    term = new Terminal({
      fontFamily: TERM_FONT,
      fontSize: termFontSize(TERM_FONT_SIZE, fontScale.level),
      theme: TERM_THEMES[theme.current],
      cursorBlink: true,
      scrollback: 5000,
      allowProposedApi: true,
    })
    // Returning false makes xterm skip the key entirely — it neither writes it to
    // the shell nor calls preventDefault, so the event stays live for the rest of
    // the app and for the WebView's own handling.
    term.attachCustomKeyEventHandler((e) => {
      if (e.type !== "keydown") return true
      // Font-size accelerators belong to the window handler in App.svelte, not the
      // shell, where Ctrl+- and Ctrl+0 would arrive as ordinary control input.
      if (fontAction(e) !== null) return false
      // Arranging panes is the grid's business, and the grid listens for the
      // bubbled keydown — returning false leaves the event live all the way up.
      if (isClosePaneChord(e) || isMaximizePaneChord(e) || isMinimizePaneChord(e)) return false
      // Moving focus between panes is the grid's business too.
      if (isFocusNextPaneChord(e) || isFocusPrevPaneChord(e)) return false
      // Window-level shortcuts (new terminal, settings, help…) belong to App.svelte.
      if (isAppShortcut(e)) return false
      const clip = clipboardAction(e, term?.hasSelection() ?? false)
      if (clip !== null) {
        // Match Windows Terminal: a copy consumes the selection, so the next Ctrl+C
        // is an interrupt again. The copy event fires after this handler returns,
        // hence the deferral.
        if (clip === "copy") setTimeout(() => term?.clearSelection(), 0)
        return false
      }
      // Chords whose modifier a plain VT stream would drop — Ctrl+Enter and
      // friends — go to ConPTY as full key events instead of xterm's encoding.
      const win32 = win32KeySequence(e)
      if (win32 === null) return true
      if (attached !== null && exitCode === undefined) {
        void writeSession(attached, encoder.encode(win32))
      }
      return false
    })
    fitAddon = new FitAddon()
    term.loadAddon(fitAddon)
    term.open(host)
    fit()

    term.onData((data) => {
      if (exitCode !== undefined) {
        if (data === "\r" || data === "\n") onrestart?.()
        return
      }
      if (attached !== null) void writeSession(attached, encoder.encode(data))
    })

    const ro = new ResizeObserver(() => fit())
    ro.observe(host)

    term.onWriteParsed(() => (dirty = true))
    const snapTimer = setInterval(snapshot, SNAPSHOT_MS)

    // The listener registration is async, so a pane unmounted before it lands
    // must still be able to cancel it.
    let unlistenDrop: (() => void) | null = null
    let dropCancelled = false
    void onFileDrop(handleDrop)
      .then((un) => (dropCancelled ? un() : (unlistenDrop = un)))
      .catch(() => {})

    return () => {
      dropCancelled = true
      unlistenDrop?.()
      clearInterval(snapTimer)
      ro.disconnect()
      disableWebgl()
      if (attached !== null) void detachSession(attached).catch(() => {})
      term?.dispose()
      term = null
      fitAddon = null
      attached = null
    }
  })

  let webgl: WebglAddon | null = null

  function enableWebgl() {
    if (webgl || !term) return
    try {
      const addon = new WebglAddon()
      addon.onContextLoss(() => {
        addon.dispose()
        if (webgl === addon) webgl = null
      })
      term.loadAddon(addon)
      webgl = addon
    } catch {
      // Without WebGL xterm falls back to its DOM renderer; not an error.
    }
  }

  function disableWebgl() {
    webgl?.dispose()
    webgl = null
  }

  // Hold a WebGL context only while the pane is on screen; a hidden layer
  // renders nothing, and the DOM renderer keeps its buffer intact meanwhile.
  $effect(() => {
    const visible = active
    if (!term) return
    if (visible) enableWebgl()
    else disableWebgl()
  })

  // On a sessionId change: detach from the old one, replay the ring buffer, attach the new one.
  $effect(() => {
    const id = sessionId
    if (!term) return
    if (attached !== null && attached !== id) {
      void detachSession(attached).catch(() => {})
      attached = null
    }
    if (id === null || attached === id) return

    let cancelled = false
    void (async () => {
      const history = await getBuffer(id)
      if (cancelled || !term) return
      term.reset()
      noticeShown = false
      // Fresh decoder per attach to avoid partial multi-byte state from a previous stream.
      const histDecoder = new TextDecoder()
      const streamDecoder = new TextDecoder()
      if (history.length > 0) term.write(histDecoder.decode(history))
      // Re-emit the exit notice after history rehydration so it is not lost on workspace
      // switch. The notice lives only in the client-side xterm buffer (not in the Rust ring
      // buffer), so it must be written again each time the pane is reattached while dead.
      if (exitCode !== undefined) {
        noticeShown = true
        term.write(exitNotice(exitCode ?? null, locale.current))
      }
      await attachSession(id, (bytes) => term?.write(streamDecoder.decode(bytes, { stream: true })))
      if (cancelled) {
        void detachSession(id).catch(() => {})
        return
      }
      attached = id
      fit()
      syncSize()
    })()

    return () => {
      cancelled = true
    }
  })

  // When the font scale changes, font size, cell count and PTY update together.
  // Assigning fontSize forces xterm to redraw with the new cell geometry;
  // fit() fixes the column/row count and syncSize() the shell's own size.
  $effect(() => {
    const size = termFontSize(TERM_FONT_SIZE, fontScale.level)
    if (!term || term.options.fontSize === size) return
    term.options.fontSize = size
    fit()
    syncSize()
  })

  // When the app theme changes, update the xterm palette live. Building a new
  // terminal would drop the scrollback and the attached session; only
  // options.theme is assigned.
  $effect(() => {
    const palette = TERM_THEMES[theme.current]
    if (!term) return
    term.options.theme = palette
  })

  // When the process dies, print the notice line once.
  $effect(() => {
    if (exitCode === undefined || noticeShown || !term) return
    noticeShown = true
    term.write(exitNotice(exitCode ?? null, locale.current))
  })
</script>

<div class="pane" class:dead={exitCode !== undefined}>
  <div class="host" bind:this={host}></div>
</div>

<style>
  .pane {
    position: relative;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    background: var(--bg-term);
  }
  .pane.dead::after {
    content: "";
    position: absolute;
    inset: 0;
    background: rgb(0 0 0 / 0.18);
    pointer-events: none;
  }
  .host {
    width: 100%;
    height: 100%;
    padding: 4px 0 0 6px;
  }
</style>
