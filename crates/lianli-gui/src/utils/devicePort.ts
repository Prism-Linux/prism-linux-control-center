/** Return the reported ENE port index. This is a controller port, not a guessed fan location. */
export function enePortIndex(deviceId: string, name: string): number | null {
  const id = deviceId.match(/:port(\d+)$/i);
  const reportedName = name.match(/\bport\s+(\d+)\b/i);
  const value = id?.[1] ?? reportedName?.[1];
  if (value === undefined) return null;
  const index = Number(value);
  return Number.isSafeInteger(index) && index >= 0 ? index : null;
}
