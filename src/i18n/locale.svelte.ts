import {
  readLocale, translate, writeLocale, type Locale, type MessageKey,
} from "./messages"

/** The active language. Reactive so that reading it through `t()` inside markup
 *  re-renders that markup when the language changes — no reload, no remount
 *  (remounting would tear down every live xterm instance). */
export const locale = $state({ current: "en" as Locale })

const store = () => (typeof localStorage === "undefined" ? undefined : localStorage)

export function initLocale() {
  apply(readLocale(store(), navigator.languages ?? [navigator.language]))
}

export function setLocale(next: Locale) {
  apply(next)
  writeLocale(store(), next)
}

function apply(next: Locale) {
  locale.current = next
  // Keeps hyphenation, spell-check and screen readers in step with the UI.
  if (typeof document !== "undefined") document.documentElement.lang = next
}

/** Translate in the active language. */
export function t(key: MessageKey, params?: Record<string, string | number>): string {
  return translate(locale.current, key, params)
}
