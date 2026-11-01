import type { DeviceInfo, LcdConfig } from "@/types";

export function resolveLcdDevice(entry: LcdConfig, devices: DeviceInfo[]): DeviceInfo | undefined {
  if (entry.serial) {
    const wanted = entry.serial.replace(/^hid:/, "");
    return devices.find((device) => device.device_id.replace(/^hid:/, "") === wanted);
  }
  return devices[entry.index ?? 0];
}

export function lcdEntryKey(entry: LcdConfig): string {
  return entry.serial ? `serial:${entry.serial.replace(/^hid:/, "")}` : `index:${entry.index ?? 0}`;
}

export function orderedLcdIndices(entries: LcdConfig[], order: string[]): number[] {
  const rank = new Map(order.map((key, position) => [key, position]));
  const rankOf = (index: number) => rank.get(lcdEntryKey(entries[index])) ?? order.length + index;
  return entries.map((_, index) => index).sort((a, b) => rankOf(a) - rankOf(b));
}

export function hasSavedLcdDevice(deviceId: string, saved: LcdConfig[], devices: DeviceInfo[]): boolean {
  return saved.some((entry) => resolveLcdDevice(entry, devices)?.device_id === deviceId);
}
