export const RECENT_LIMIT = 20

export function pushRecent(recents: string[], path: string): string[] {
  return [path, ...recents.filter((p) => p !== path)].slice(0, RECENT_LIMIT)
}
