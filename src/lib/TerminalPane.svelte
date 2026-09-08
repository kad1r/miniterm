<script lang="ts">
  import { onMount } from "svelte"
  import { Terminal } from "@xterm/xterm"
  import { FitAddon } from "@xterm/addon-fit"
  import { WebglAddon } from "@xterm/addon-webgl"
  import "@xterm/xterm/css/xterm.css"
  import { attachSession, detachSession, getBuffer, resizeSession, writeSession } from "../ipc"
  import { exitNotice } from "../term/exit"
  import { TERM_FONT, TERM_FONT_SIZE, TERM_THEME } from "../term/theme"

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

  onMount(() => {
    term = new Terminal({
      fontFamily: TERM_FONT,
      fontSize: TERM_FONT_SIZE,
      theme: TERM_THEME,
      cursorBlink: true,
      scrollback: 5000,
      allowProposedApi: true,
    })
    fitAddon = new FitAddon()
    term.loadAddon(fitAddon)
    term.open(host)
    try {
      term.loadAddon(new WebglAddon())
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

    return () => {
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
        term.write(exitNotice(exitCode ?? null))
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

  // Süreç ölünce bilgi satırını bir kez bas.
  $effect(() => {
    if (exitCode === undefined || noticeShown || !term) return
    noticeShown = true
    term.write(exitNotice(exitCode ?? null))
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
