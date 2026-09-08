export function exitNotice(code: number | null): string {
  const shown = code === null ? "bilinmiyor" : String(code)
  return `\r\n\x1b[90m[process exited: ${shown}] — yeniden başlatmak için Enter\x1b[0m\r\n`
}
