import { i18n, tr, type MessageKey } from "@/i18n";
import labels from "@/i18n/locales/en/findings/labels";
import type { InstallationFinding } from "@/types/installation";

// Features and contexts are a fixed English vocabulary; unknown values are shown as sent.
function byEnglish(group: "feature" | "context"): Map<string, MessageKey> {
  return new Map(
    Object.entries(labels[group]).map(([key, english]) => [english, `findings.${group}.${key}` as MessageKey]),
  );
}

const FEATURES = byEnglish("feature");
const CONTEXTS = byEnglish("context");

function messageText(finding: InstallationFinding, part: "title" | "remediation", english: string): string {
  if (!finding.message_key || !english) return english;
  const key = `findings.${finding.message_key}.${part}`;
  return i18n.global.te(key) ? i18n.global.t(key, finding.message_params ?? {}) : english;
}

export function findingTitle(finding: InstallationFinding): string {
  return messageText(finding, "title", finding.title);
}

export function findingRemediation(finding: InstallationFinding): string {
  return messageText(finding, "remediation", finding.remediation);
}

export function findingFeature(finding: InstallationFinding): string {
  const key = FEATURES.get(finding.feature);
  return key ? tr(key) : finding.feature;
}

// Contexts the producers build with format!; a nested `context` is translated in turn.
const CONTEXT_PATTERNS: [RegExp, MessageKey, string[]][] = [
  [/^Desktop user: (.+)$/s, "findings.contextPattern.desktopUser", ["context"]],
  [/^Daemon UID (\d+): (.+)$/s, "findings.contextPattern.daemonUid", ["uid", "context"]],
  [/^Host account (\d+)$/, "findings.contextPattern.hostAccountUid", ["uid"]],
  [/^Native process UID (\d+)$/, "findings.contextPattern.nativeProcessUid", ["uid"]],
  [/^Distrobox (.+), process UID (\d+)$/, "findings.contextPattern.distroboxProcessUid", ["name", "uid"]],
  [/^Unsupported container, UID (\d+)$/, "findings.contextPattern.unsupportedContainerUid", ["uid"]],
  [/^Distrobox (.+)$/, "findings.contextPattern.distrobox", ["name"]],
  [/^Process UID (\d+)$/, "findings.contextPattern.processUid", ["uid"]],
  [/^Effective UID (\d+), HID backend (.+)$/, "findings.contextPattern.effectiveUidBackend", ["uid", "backend"]],
];

function contextText(context: string): string {
  const key = CONTEXTS.get(context);
  if (key) return tr(key);
  for (const [pattern, messageKey, names] of CONTEXT_PATTERNS) {
    const match = pattern.exec(context);
    if (!match) continue;
    const params = Object.fromEntries(
      names.map((name, index) => [name, name === "context" ? contextText(match[index + 1]) : match[index + 1]]),
    );
    return tr(messageKey, params);
  }
  return context;
}

export function findingContext(finding: InstallationFinding): string {
  return contextText(finding.context);
}
