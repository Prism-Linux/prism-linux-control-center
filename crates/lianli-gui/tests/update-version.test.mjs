import test from "node:test";
import assert from "node:assert/strict";
import { compareVersions, isNewer, isPrerelease, latestReleaseVersion, normalizeVersion, releaseApiForVersion, releaseUrlForVersion, usesPrismReleaseChannel } from "../src/utils/releaseVersion.ts";

test("normalizes stable and beta release tags", () => {
  assert.equal(normalizeVersion("v1.3.0-beta.1"), "1.3.0-beta.1");
  assert.equal(normalizeVersion("1.3.0"), "1.3.0");
  assert.equal(normalizeVersion("01.3.0"), null);
  assert.equal(normalizeVersion("1.3.0-beta.01"), null);
});

test("orders beta releases after 1.2.4 and before their stable release", () => {
  assert.equal(isNewer("1.3.0-beta.1", "1.2.4"), true);
  assert.equal(isNewer("1.3.0-beta.2", "1.3.0-beta.1"), true);
  assert.equal(isNewer("1.3.0", "1.3.0-beta.2"), true);
  assert.equal(isNewer("1.3.0-beta.1", "1.3.0"), false);
  assert.equal(compareVersions("1.3.0-beta.10", "1.3.0-beta.2"), 1);
});

test("only beta builds use the prerelease update channel", () => {
  assert.equal(isPrerelease("1.3.0-beta.1"), true);
  assert.equal(isPrerelease("1.3.0"), false);
  assert.equal(isPrerelease("not-a-version"), false);
  assert.equal(usesPrismReleaseChannel("1.2.4"), false);
  assert.equal(usesPrismReleaseChannel("1.3.0-beta.1"), true);
  assert.equal(usesPrismReleaseChannel("1.3.0"), true);
  assert.equal(releaseApiForVersion("1.2.4", "legacy stable", "Prism stable", "Prism prerelease"), "legacy stable");
  assert.equal(releaseApiForVersion("1.3.0-beta.1", "legacy stable", "Prism stable", "Prism prerelease"), "Prism prerelease");
  assert.equal(releaseApiForVersion("1.3.0", "legacy stable", "Prism stable", "Prism prerelease"), "Prism stable");
  assert.equal(releaseUrlForVersion("1.2.4", "legacy stable releases", "Prism releases"), "legacy stable releases");
  assert.equal(releaseUrlForVersion("1.3.0", "legacy stable releases", "Prism releases"), "Prism releases");
});

test("prerelease channel ignores drafts and selects the highest stable or beta tag", () => {
  assert.equal(latestReleaseVersion([
    { tag_name: "v1.3.0-beta.2", draft: false, prerelease: true },
    { tag_name: "v1.3.0", draft: false, prerelease: false },
    { tag_name: "v2.0.0-beta.1", draft: true, prerelease: true },
    { tag_name: "draft-label", draft: false, prerelease: false },
  ]), "1.3.0");
  assert.equal(latestReleaseVersion([{ tag_name: "v1.3.0-beta.2", draft: false }]), "1.3.0-beta.2");
});
