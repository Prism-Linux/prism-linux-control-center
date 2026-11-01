import { invoke as tauriInvoke, type InvokeArgs } from "@tauri-apps/api/core";
import { i18n } from "@/i18n";
import { serializeInvokeArgs } from "@/utils/plainData";

/** A message key under `backend.*` sent by the daemon or the GUI backend beside its English text. */
export interface UiMessage {
  key: string;
  params?: Record<string, string>;
}

export function backendText(english: string, ui?: UiMessage | null): string {
  if (!ui) return english;
  const key = `backend.${ui.key}`;
  return i18n.global.te(key) ? i18n.global.t(key, ui.params ?? {}) : english;
}

function errorText(error: unknown): string {
  if (error && typeof error === "object" && "message" in error && typeof error.message === "string") {
    const ui = "ui" in error ? (error.ui as UiMessage | null | undefined) : null;
    return backendText(error.message, ui);
  }
  return String(error);
}

/** Tauri's invoke; a structured command error is rethrown as its translated text. */
export async function invoke<T>(command: string, args?: InvokeArgs): Promise<T> {
  try {
    return await tauriInvoke<T>(command, serializeInvokeArgs(args as Record<string, unknown> | undefined) as InvokeArgs);
  } catch (error) {
    throw errorText(error);
  }
}
