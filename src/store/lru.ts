/** Global Constraint: maximum number of workspaces with live xterm instances.
 *  Raising this increases RAM usage proportionally (one xterm DOM tree per slot). */
export const LIVE_LIMIT = 3;

export function touch(
  list: string[],
  id: string,
  max: number,
): { list: string[]; evicted: string[] } {
  const next = [id, ...list.filter((x) => x !== id)]
  return { list: next.slice(0, max), evicted: next.slice(max) }
}
