export interface ReleaseVersion {
  major: number;
  minor: number;
  patch: number;
  prerelease: string[];
}

export function parseVersion(value: string): ReleaseVersion | null {
  const match = /^v?(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$/.exec(value);
  if (!match) return null;
  const prerelease = match[4]?.split(".") ?? [];
  if (prerelease.some((part) => /^\d+$/.test(part) && part.length > 1 && part.startsWith("0"))) return null;
  return { major: Number(match[1]), minor: Number(match[2]), patch: Number(match[3]), prerelease };
}

export function normalizeVersion(value: string): string | null {
  const parsed = parseVersion(value);
  if (!parsed) return null;
  const core = `${parsed.major}.${parsed.minor}.${parsed.patch}`;
  return parsed.prerelease.length ? `${core}-${parsed.prerelease.join(".")}` : core;
}

export function isPrerelease(value: string): boolean {
  const parsed = parseVersion(value);
  return parsed !== null && parsed.prerelease.length !== 0;
}

export function usesPrismReleaseChannel(version: string): boolean {
  const comparison = compareVersions(version, "1.3.0-beta.1");
  return comparison !== null && comparison >= 0;
}

export function releaseApiForVersion(version: string, legacyStableUrl: string, prismStableUrl: string, prereleasesUrl: string): string {
  if (isPrerelease(version)) return prereleasesUrl;
  return usesPrismReleaseChannel(version) ? prismStableUrl : legacyStableUrl;
}

export function releaseUrlForVersion(version: string, legacyStableUrl: string, prismUrl: string): string {
  return usesPrismReleaseChannel(version) ? prismUrl : legacyStableUrl;
}

function comparePrerelease(left: string[], right: string[]): number {
  if (left.length === 0 || right.length === 0) {
    return left.length === right.length ? 0 : left.length === 0 ? 1 : -1;
  }
  const count = Math.min(left.length, right.length);
  for (let i = 0; i < count; i++) {
    const a = left[i];
    const b = right[i];
    if (a === b) continue;
    const aNumeric = /^\d+$/.test(a);
    const bNumeric = /^\d+$/.test(b);
    if (aNumeric && bNumeric) return Number(a) < Number(b) ? -1 : 1;
    if (aNumeric !== bNumeric) return aNumeric ? -1 : 1;
    return a < b ? -1 : 1;
  }
  return left.length === right.length ? 0 : left.length < right.length ? -1 : 1;
}

export function compareVersions(left: string, right: string): number | null {
  const a = parseVersion(left);
  const b = parseVersion(right);
  if (!a || !b) return null;
  for (const field of ["major", "minor", "patch"] as const) {
    if (a[field] !== b[field]) return a[field] < b[field] ? -1 : 1;
  }
  return comparePrerelease(a.prerelease, b.prerelease);
}

export function isNewer(candidate: string, current: string): boolean {
  const comparison = compareVersions(candidate, current);
  return comparison !== null && comparison > 0;
}

export function latestReleaseVersion(releases: unknown[]): string | null {
  const candidates = releases.flatMap((value) => {
    if (!value || typeof value !== "object") return [];
    const release = value as { tag_name?: unknown; draft?: unknown };
    if (release.draft || typeof release.tag_name !== "string") return [];
    const version = normalizeVersion(release.tag_name);
    return version ? [version] : [];
  });
  return candidates.reduce<string | null>((latest, candidate) =>
    latest === null || isNewer(candidate, latest) ? candidate : latest, null);
}
