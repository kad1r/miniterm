export const MAX_TERMINALS = 6;
export const MIN_CELL_PX = 120;

export interface GridLayout {
  rows: number;
  cols: number;
}

/** Every factor pair of the terminal count, by ascending row count. */
export function layoutsFor(count: number): GridLayout[] {
  if (!Number.isInteger(count) || count < 1 || count > MAX_TERMINALS) return [];
  const out: GridLayout[] = [];
  for (let rows = 1; rows <= count; rows++) {
    if (count % rows === 0) out.push({ rows, cols: count / rows });
  }
  return out;
}

export function equalSizes(n: number): number[] {
  return Array.from({ length: n }, () => 1 / n);
}

/**
 * Moves divider `dividerIndex` by `deltaPx`: space changes hands only between
 * `sizes[dividerIndex]` and `sizes[dividerIndex + 1]`, so the total stays 1
 * and no other cell moves.
 */
export function resizeFractions(
  sizes: number[],
  dividerIndex: number,
  deltaPx: number,
  totalPx: number,
  minPx = MIN_CELL_PX,
): number[] {
  if (dividerIndex < 0 || dividerIndex >= sizes.length - 1 || totalPx <= 0) return sizes;

  const minFraction = minPx / totalPx;
  const a = sizes[dividerIndex];
  const b = sizes[dividerIndex + 1];
  const pair = a + b;

  if (pair < minFraction * 2) return sizes;

  let delta = deltaPx / totalPx;
  delta = Math.max(delta, minFraction - a);
  delta = Math.min(delta, b - minFraction);

  const next = [...sizes];
  next[dividerIndex] = a + delta;
  next[dividerIndex + 1] = b - delta;
  return next;
}

export function toGridTemplate(sizes: number[]): string {
  return sizes.map((s) => `${s}fr`).join(" ");
}
