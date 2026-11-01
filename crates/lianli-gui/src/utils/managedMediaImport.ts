import type { DaemonInfo, LcdConfig, LcdTemplate } from "@/types";
import type { UiMessage } from "@/utils/backendText";
import { toPlainData } from "./plainData.ts";

export type ManagedImportProgress = "authorizing" | "copying";

export interface ManagedImportStatus {
  id: string;
  active: boolean;
  result: { import_id: string; lcds: LcdConfig[]; templates: LcdTemplate[] } | null;
  error: string | null;
  error_ui?: UiMessage | null;
}

export interface PublishedSelection {
  import_id: string;
  lcds: LcdConfig[];
  templates: LcdTemplate[];
}

export interface ManagedImportApi {
  start(instance: string, lcds: LcdConfig[], templates: LcdTemplate[]): Promise<string>;
  status(): Promise<ManagedImportStatus | null>;
  result(instance: string, id: string): Promise<PublishedSelection>;
  verify(instance: string, lcds: LcdConfig[]): Promise<boolean>;
}

export class ManagedImportError extends Error {
  readonly ui?: UiMessage | null;

  constructor(message: string, ui?: UiMessage | null) {
    super(message);
    this.name = "ManagedImportError";
    this.ui = ui;
  }
}

export function shouldOfferLcdSave(
  lcd: LcdConfig,
  hasDraftChanges: boolean,
  mediaState: "waiting_for_device" | "preparing" | "ready" | "failed" | undefined,
): boolean {
  return hasDraftChanges || (["image", "video", "gif"].includes(lcd.type) && mediaState === "failed");
}

export async function sha256Hex(bytes: ArrayBuffer): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("");
}

export function isManagedImportPath(path: string | null | undefined, configPath: string | undefined): boolean {
  if (!path || !configPath || !path.startsWith("/") || !configPath.startsWith("/")) return false;
  const configSeparator = configPath.lastIndexOf("/");
  if (configSeparator <= 0) return false;
  const stateDirectory = configPath.slice(0, configSeparator);
  const prefix = `${stateDirectory}/media/imports/`;
  if (!path.startsWith(prefix)) return false;
  const relative = path.slice(prefix.length).split("/");
  return relative.length === 2
    && /^[a-f0-9]{32}$/i.test(relative[0])
    && relative[1] !== ""
    && relative[1] !== "."
    && relative[1] !== "..";
}

export async function copyLcdMediaToManaged(
  lcd: LcdConfig,
  daemon: DaemonInfo,
  api: ManagedImportApi,
  onProgress: (progress: ManagedImportProgress) => void,
  options: { intervalMs?: number; timeoutMs?: number; wait?: (milliseconds: number) => Promise<void>; sourceSha256?: string } = {},
): Promise<LcdConfig> {
  if (!lcd.path) throw new ManagedImportError("Choose a media file before copying it into managed storage.");
  if (isManagedImportPath(lcd.path, daemon.config_path)) return lcd;

  // A previous import may have completed while saving its config failed. Reuse that
  // publication only when the source digest, LCD identity, managed-path shape, and
  // live daemon readability all match.
  if (options.sourceSha256) {
    try {
      const previous = await api.status();
      if (previous && !previous.active && !previous.error && previous.result
        && previous.result.import_id === previous.id && /^[a-f0-9]{32}$/i.test(previous.id)) {
        const published = await api.result(daemon.instance_id, previous.id);
        const reused = published.lcds.find((candidate) =>
          candidate.serial === lcd.serial
          && (candidate.index ?? null) === (lcd.index ?? null)
          && candidate.type === lcd.type
          && isManagedImportPath(candidate.path, daemon.config_path)
          && candidate.path?.split("/").slice(-1)[0]?.toLowerCase().startsWith(`${options.sourceSha256?.toLowerCase()}.`),
        );
        if (published.import_id === previous.id && reused && await api.verify(daemon.instance_id, [reused])) {
          return toPlainData(reused);
        }
      }
    } catch {
      // Reuse is an optimization. If a prior result cannot be verified, perform a
      // fresh authorized import and let its errors follow the normal Save path.
    }
  }

  onProgress("authorizing");
  const id = await api.start(daemon.instance_id, [toPlainData(lcd)], []);
  if (!/^[a-f0-9]{32}$/i.test(id)) throw new ManagedImportError("The media import returned an invalid operation ID.");

  const intervalMs = options.intervalMs ?? 500;
  const timeoutMs = options.timeoutMs ?? 780_000;
  const wait = options.wait ?? ((milliseconds: number) => new Promise<void>((resolve) => setTimeout(resolve, milliseconds)));
  const deadline = Date.now() + timeoutMs;

  while (Date.now() <= deadline) {
    const status = await api.status();
    if (status?.id === id) {
      if (status.error) throw new ManagedImportError(status.error, status.error_ui);
      if (!status.active) {
        if (!status.result || status.result.import_id !== id) {
          throw new ManagedImportError("The media import ended without a verified result. Check managed storage before retrying.");
        }
        const published = await api.result(daemon.instance_id, id);
        if (published.import_id !== id || published.lcds.length !== 1) {
          throw new ManagedImportError("The media import returned a different LCD selection.");
        }
        const imported = published.lcds[0];
        if (imported.serial !== lcd.serial || (imported.index ?? null) !== (lcd.index ?? null) || imported.type !== lcd.type
          || !isManagedImportPath(imported.path, daemon.config_path)) {
          throw new ManagedImportError("The media import did not return a managed copy for this LCD.");
        }
        if (!await api.verify(daemon.instance_id, [imported])) {
          throw new ManagedImportError("The managed copy was published, but the system daemon cannot read it. The configuration was not saved; retry Save or inspect managed storage.");
        }
        return imported;
      }
      onProgress("copying");
    }
    if (Date.now() >= deadline) break;
    await wait(intervalMs);
  }

  throw new ManagedImportError("Copying this media is taking too long. Check import progress before retrying.");
}
