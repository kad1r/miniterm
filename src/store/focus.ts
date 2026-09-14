/**
 * Where the keyboard lands after the pane at `closed` is removed and the panes
 * behind it slide down a cell.
 *
 * Closing the focused pane keeps the index, which is now the pane that took its
 * place — the same thing Windows Terminal does. Closing the *last* pane has no
 * successor, so the clamp walks focus back one cell instead.
 */
export function focusAfterClose(focused: number, closed: number, remaining: number): number {
  if (remaining <= 0) return 0;
  const shifted = focused > closed ? focused - 1 : focused;
  return Math.min(Math.max(shifted, 0), remaining - 1);
}

/**
 * The same slide applied to the maximized pane. Unlike focus, a maximized pane
 * that is itself removed has no successor: blowing an unrelated pane up to full
 * screen because its neighbour was closed would be the app moving on its own.
 */
export function maximizedAfterClose(maximized: number | null, closed: number): number | null {
  if (maximized === null) return null;
  if (maximized === closed) return null;
  return maximized > closed ? maximized - 1 : maximized;
}
