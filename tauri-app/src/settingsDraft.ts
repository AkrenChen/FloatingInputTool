export function mergeSettingsDraft<T extends object>(
  current: T, incoming: T, dirtyKeys: ReadonlySet<string>,
): T {
  const result = { ...incoming }
  for (const key of Object.keys(current) as Array<keyof T>) {
    if (dirtyKeys.has(String(key))) result[key] = current[key]
  }
  return result
}
