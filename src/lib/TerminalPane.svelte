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
  import { exitNotice } from "../term/exit"
  import { dropText } from "../term/paths"
  import { locale } from "../i18n/locale.svelte"
  import { TERM_FONT, TERM_FONT_SIZE, TERM_THEME } from "../term/theme"
  import { fontAction, termFontSize } from "../store/font"
  import { fontScale } from "../store/font.svelte"

  let { sessionId, exitCode = undefined, onrestart }: {
    sessionId: number | null
    exitCode?: number | null | undefined
    onrestart?: () => void
  } = $props()

  let host: HTMLDivElement
  let term: Terminal | null = null
  let fitAddon: FitAddon | null = null
  let attached: number | null = null
  let noticeShown = false
  const encoder = new TextEncoder()

  /** Yalnızca DOM ölçüsünü günceller; PTY'ye dokunmaz. Sürükleme sırasında güvenli. */
  export function fit() {
    if (!term || !fitAddon || !host?.isConnected || host.clientWidth < 2) return
    fitAddon.fit()
  }

  /** PTY boyutunu son fit()'e eşitler. Sürükleme bitince bir kez çağrılır. */
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
    const text = dropText(drop.paths)
    if (!text) return
    term.focus()
    void writeSession(attached, encoder.encode(text))
  }

  onMount(() => {
    term = new Terminal({
      fontFamily: TERM_FONT,
      fontSize: termFontSize(TERM_FONT_SIZE, fontScale.level),
      theme: TERM_THEME,
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
      const clip = clipboardAction(e, term?.hasSelection() ?? false)
      if (clip === null) return true
      // Match Windows Terminal: a copy consumes the selection, so the next Ctrl+C
      // is an interrupt again. The copy event fires after this handler returns,
      // hence the deferral.
      if (clip === "copy") setTimeout(() => term?.clearSelection(), 0)
      return false
    })
    fitAddon = new FitAddon()
    term.loadAddon(fitAddon)
    term.open(host)
    try {
      const webgl = new WebglAddon()
      webgl.onContextLoss(() => webgl.dispose())
      term.loadAddon(webgl)
    } catch {
      // WebGL yoksa xterm canvas/DOM renderer'a düşer; hata değil.
    }
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
      ro.disconnect()
      if (attached !== null) void detachSession(attached).catch(() => {})
      term?.dispose()
      term = null
      fitAddon = null
      attached = null
    }
  })

  // sessionId değişince: eskiden ayrıl, ring buffer'ı bas, yenisine bağlan.
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

  // Yazı ölçeği değişince punto, hücre sayısı ve PTY birlikte güncellenir.
  // fontSize ataması xterm'i yeni hücre geometrisiyle baştan çizmeye zorlar;
  // fit() sütun/satır sayısını, syncSize() de kabuğun kendi boyutunu düzeltir.
  $effect(() => {
    const size = termFontSize(TERM_FONT_SIZE, fontScale.level)
    if (!term || term.options.fontSize === size) return
    term.options.fontSize = size
    fit()
    syncSize()
  })

  // Süreç ölünce bilgi satırını bir kez bas.
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
    background: var(--bg);
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
