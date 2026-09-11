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
