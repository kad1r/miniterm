import { translate, type Locale } from "../i18n/messages"

export function exitNotice(code: number | null, locale: Locale): string {
  const shown = code === null ? translate(locale, "term.exitUnknown") : String(code)
  return `\r\n\x1b[90m${translate(locale, "term.exitNotice", { code: shown })}\x1b[0m\r\n`
}
