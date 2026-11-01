import { toRaw } from "vue";

const OMIT = Symbol("omit-undefined-property");

/**
 * Convert reactive/plain application data to JSON-compatible plain values before
 * cloning it or sending it over Tauri IPC. `structuredClone` rejects Vue proxies.
 */
export function toPlainData<T>(value: T): T {
  const ancestors = new WeakSet<object>();

  function visit(input: unknown, path: string): unknown | typeof OMIT {
    if (input === undefined) return OMIT;
    if (input === null || typeof input === "string" || typeof input === "boolean") return input;
    if (typeof input === "number") {
      if (!Number.isFinite(input)) throw new TypeError(`Non-finite number at ${path} cannot be sent over IPC.`);
      return input;
    }
    if (typeof input !== "object") {
      throw new TypeError(`Non-serializable value at ${path} cannot be sent over IPC.`);
    }

    const raw = toRaw(input);
    if (ancestors.has(raw)) throw new TypeError(`Circular value at ${path} cannot be sent over IPC.`);
    ancestors.add(raw);
    try {
      if (Array.isArray(raw)) {
        return raw.map((item, index) => {
          const value = visit(item, `${path}[${index}]`);
          return value === OMIT ? null : value;
        });
      }

      const prototype = Object.getPrototypeOf(raw);
      if (prototype !== Object.prototype && prototype !== null) {
        throw new TypeError(`Unsupported object at ${path} cannot be sent over IPC.`);
      }
      const result: Record<string, unknown> = {};
      for (const key of Object.keys(raw)) {
        const value = visit((raw as Record<string, unknown>)[key], `${path}.${key}`);
        if (value !== OMIT) {
          Object.defineProperty(result, key, { value, enumerable: true, configurable: true, writable: true });
        }
      }
      return result;
    } finally {
      ancestors.delete(raw);
    }
  }

  const plain = visit(value, "$");
  if (plain === OMIT) throw new TypeError("Undefined cannot be sent over IPC.");
  return plain as T;
}

export function serializeInvokeArgs<T extends Record<string, unknown> | undefined>(args: T): T {
  return args === undefined ? args : toPlainData(args);
}
