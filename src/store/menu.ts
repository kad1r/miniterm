export interface MenuItem {
  label: string
  action: () => void
  danger?: boolean
}

/**
 * Invoke a context-menu item and then dismiss the menu.
 *
 * The order matters and is not stylistic. Dismissing first unmounts the button
 * that is still mid-click, and WebView2 silently suppresses any modal script
 * dialog (`prompt`, `confirm`) opened while its invoking element is being torn
 * down — no dialog, no error, the call just returns null. That is how rename
 * and delete came to be dead from the context menu while the same functions
 * worked from the F2 shortcut, which tears nothing down.
 *
 * `finally` covers the dismissal so a throwing action cannot strand an open
 * menu over the tree.
 */
export function pickItem(item: MenuItem, onclose: () => void): void {
  try {
    item.action()
  } finally {
    onclose()
  }
}
