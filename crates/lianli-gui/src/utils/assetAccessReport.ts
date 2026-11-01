import type { AssetAccessReport } from "@/types";
import type { UiMessage } from "@/utils/backendText";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isCount(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}

function parseUiMessage(value: unknown): UiMessage | null | undefined {
  if (value === undefined || value === null) return value;
  if (!isRecord(value) || typeof value.key !== "string") throw new Error("Invalid media-access diagnostic message.");
  if (value.params !== undefined && value.params !== null) {
    if (!isRecord(value.params) || Object.values(value.params).some((item) => typeof item !== "string")) {
      throw new Error("Invalid media-access diagnostic message parameters.");
    }
    return { key: value.key, params: value.params as Record<string, string> };
  }
  return { key: value.key };
}

export function parseAssetAccessReport(value: unknown): AssetAccessReport {
  if (!isRecord(value) || !isCount(value.uid) || !isCount(value.checked) || !isCount(value.failed)
    || !Array.isArray(value.issues) || value.issues.length > 32 || value.issues.length > value.failed) {
    throw new Error("The daemon returned an invalid media-access report.");
  }

  const issues = value.issues.map((issue): AssetAccessReport["issues"][number] => {
    if (!isRecord(issue) || typeof issue.owner !== "string" || typeof issue.error !== "string"
      || (issue.path !== null && typeof issue.path !== "string")) {
      throw new Error("The daemon returned an invalid media-access issue.");
    }
    return {
      owner: issue.owner,
      owner_ui: parseUiMessage(issue.owner_ui),
      path: issue.path,
      error: issue.error,
      error_ui: parseUiMessage(issue.error_ui),
    };
  });

  return { uid: value.uid, checked: value.checked, failed: value.failed, issues };
}
