import type { RgbDeviceCapabilities } from "@/types";

export interface NamedDevice {
  device_id: string;
  name: string;
}

export function deviceNameKey(deviceId: string): string {
  // ENE RGB groups are the lighting side of the fan port with the same index.
  return deviceId.replace(/^hid:/, "").replace(/:group(\d+)$/, ":port$1");
}

export function deviceLabels(devices: NamedDevice[]): Map<string, string> {
  const sorted = [...devices].sort((a, b) => a.device_id.localeCompare(b.device_id));
  const totals = new Map<string, number>();
  for (const device of sorted) totals.set(device.name, (totals.get(device.name) ?? 0) + 1);
  const seen = new Map<string, number>();
  const labels = new Map<string, string>();
  for (const device of sorted) {
    const n = (seen.get(device.name) ?? 0) + 1;
    seen.set(device.name, n);
    labels.set(device.device_id, (totals.get(device.name) ?? 1) > 1 ? `${device.name} #${n}` : device.name);
  }
  return labels;
}

export function rgbDeviceLabels(
  caps: RgbDeviceCapabilities[],
  deviceWide: Map<string, string>,
): Map<string, string> {
  const local = deviceLabels(caps.map((cap) => ({ device_id: cap.device_id, name: cap.device_name })));
  const wide = (id: string) => deviceWide.get(id) ?? deviceWide.get(id.replace(/:group(\d+)$/, ":port$1"));
  return new Map(caps.map((cap) => [cap.device_id, wide(cap.device_id) ?? local.get(cap.device_id) ?? cap.device_name]));
}
