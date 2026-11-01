import test from "node:test";
import assert from "node:assert/strict";
import { parseAssetAccessReport } from "../src/utils/assetAccessReport.ts";

test("accepts a bounded daemon report with optional translated details", () => {
  const report = parseAssetAccessReport({
    uid: 984,
    checked: 1,
    failed: 1,
    issues: [{
      owner: "LCD image",
      owner_ui: { key: "mediaAccess.lcdImage" },
      path: "/media/image.png",
      error: "Permission denied",
      error_ui: null,
    }],
  });

  assert.equal(report.issues[0].path, "/media/image.png");
  assert.equal(report.issues[0].owner_ui?.key, "mediaAccess.lcdImage");
});

test("rejects null or incomplete IPC responses before they reach the template", () => {
  assert.throws(() => parseAssetAccessReport(null), /invalid media-access report/i);
  assert.throws(() => parseAssetAccessReport({ uid: 984, checked: 1, failed: 1, issues: [null] }), /invalid media-access issue/i);
});

test("rejects malformed translated params and inconsistent issue lists", () => {
  assert.throws(() => parseAssetAccessReport({
    uid: 984, checked: 1, failed: 1,
    issues: [{ owner: "LCD", error: "Denied", path: null, error_ui: { key: "x", params: { count: 1 } } }],
  }), /invalid media-access diagnostic message parameters/i);
  assert.throws(() => parseAssetAccessReport({ uid: 984, checked: 0, failed: 0, issues: [{ owner: "LCD", error: "Denied", path: null }] }), /invalid media-access report/i);
});
