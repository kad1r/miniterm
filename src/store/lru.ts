export function touch(
  list: string[],
  id: string,
  max: number,
): { list: string[]; evicted: string[] } {
  const next = [id, ...list.filter((x) => x !== id)]
  return { list: next.slice(0, max), evicted: next.slice(max) }
}
