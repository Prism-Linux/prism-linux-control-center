import { defineStore } from "pinia";
import { computed, ref } from "vue";
import type { LcdConfig } from "@/types";
import { useConfigStore } from "@/stores/config";
import { useDevicesStore } from "@/stores/devices";
import { deviceNameKey } from "@/utils/deviceNames";
import { resolveLcdDevice } from "@/utils/lcdSelection";
import { readStoredJson, writeStoredJson } from "@/utils/storedPreference";

const NAMES_KEY = "lianli-linux.deviceNames";
const LEGACY_LCD_NAMES_KEY = "lianli-linux.lcdNames";
export const MAX_DEVICE_NAME_LENGTH = 40;

function stringMap(value: unknown): Record<string, string> | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  return Object.fromEntries(
    Object.entries(value).filter((entry): entry is [string, string] => typeof entry[1] === "string"),
  );
}

function migrateLcdNames(): Record<string, string> {
  const legacy = stringMap(readStoredJson(LEGACY_LCD_NAMES_KEY)) ?? {};
  return Object.fromEntries(
    Object.entries(legacy).map(([key, name]) => [
      key.startsWith("serial:") ? deviceNameKey(key.slice("serial:".length)) : key,
      name,
    ]),
  );
}

function readNames(): Record<string, string> {
  const stored = stringMap(readStoredJson(NAMES_KEY));
  if (stored) return stored;
  const migrated = migrateLcdNames();
  writeStoredJson(NAMES_KEY, migrated);
  return migrated;
}

function indexKey(entry: LcdConfig): string {
  return `index:${entry.index ?? 0}`;
}

function assignName(target: Record<string, string>, key: string, name: string) {
  const trimmed = name.trim().slice(0, MAX_DEVICE_NAME_LENGTH);
  if (trimmed) target[key] = trimmed;
  else delete target[key];
}

export const useDeviceNamesStore = defineStore("deviceNames", () => {
  const config = useConfigStore();
  const devices = useDevicesStore();
  const names = ref<Record<string, string>>(readNames());
  const hasIndexNames = computed(() => Object.keys(names.value).some((key) => key.startsWith("index:")));

  function write(next: Record<string, string>) {
    names.value = next;
    writeStoredJson(NAMES_KEY, next);
  }

  function indexEntriesFor(deviceId: string): LcdConfig[] {
    if (!hasIndexNames.value) return [];
    const key = deviceNameKey(deviceId);
    return config.config.lcds.filter((entry) => {
      if (entry.serial) return false;
      const device = resolveLcdDevice(entry, devices.lcdDevices);
      return !!device && deviceNameKey(device.device_id) === key;
    });
  }

  function nameOf(deviceId: string): string {
    const own = names.value[deviceNameKey(deviceId)];
    if (own) return own;
    for (const entry of indexEntriesFor(deviceId)) {
      const legacy = names.value[indexKey(entry)];
      if (legacy) return legacy;
    }
    return "";
  }

  function label(deviceId: string, fallback: string): string {
    return nameOf(deviceId) || fallback;
  }

  function setName(deviceId: string, name: string) {
    const next = { ...names.value };
    for (const entry of indexEntriesFor(deviceId)) delete next[indexKey(entry)];
    assignName(next, deviceNameKey(deviceId), name);
    write(next);
  }

  function lcdEntryName(entry: LcdConfig): string {
    if (entry.serial) return names.value[deviceNameKey(entry.serial)] ?? "";
    const device = resolveLcdDevice(entry, devices.lcdDevices);
    return (device && names.value[deviceNameKey(device.device_id)]) || names.value[indexKey(entry)] || "";
  }

  function setLcdEntryName(entry: LcdConfig, name: string) {
    const device = entry.serial ? undefined : resolveLcdDevice(entry, devices.lcdDevices);
    const deviceId = entry.serial || device?.device_id;
    if (deviceId) {
      setName(deviceId, name);
      return;
    }
    const next = { ...names.value };
    assignName(next, indexKey(entry), name);
    write(next);
  }

  return { names, nameOf, label, setName, lcdEntryName, setLcdEntryName };
});
