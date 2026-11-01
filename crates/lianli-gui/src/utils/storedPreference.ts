const RENAMED_KEYS = ["theme", "deviceNames", "lcdNames", "lcdOrder"];

// Must run before any store reads these keys.
export function migrateStoredKeys() {
  for (const name of RENAMED_KEYS) {
    try {
      const legacy = localStorage.getItem(`lianli.${name}`);
      if (legacy === null) continue;
      if (localStorage.getItem(`lianli-linux.${name}`) === null) {
        localStorage.setItem(`lianli-linux.${name}`, legacy);
      }
      localStorage.removeItem(`lianli.${name}`);
    } catch {
      // Storage unavailable; the next start retries.
    }
  }
}

export function readStoredJson(key: string): unknown {
  try {
    return JSON.parse(localStorage.getItem(key) ?? "null");
  } catch {
    return null;
  }
}

export function writeStoredJson(key: string, value: unknown) {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // Display preferences stay in memory when storage is unavailable.
  }
}
