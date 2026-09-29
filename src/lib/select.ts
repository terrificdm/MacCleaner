/** Keep the first item for each path (modules can list the same folder). */
export function uniqueByPath<T extends { path: string }>(items: T[]): T[] {
  const seen = new Set<string>();
  return items.filter((i) => (seen.has(i.path) ? false : (seen.add(i.path), true)));
}
