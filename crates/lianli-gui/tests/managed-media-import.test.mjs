import test from "node:test";
import assert from "node:assert/strict";
import { copyLcdMediaToManaged, isManagedImportPath, sha256Hex, shouldOfferLcdSave } from "../src/utils/managedMediaImport.ts";

const daemon = { instance_id: "instance", config_path: "/var/lib/lianli/config.json" };
const lcd = { serial: "hid:1cbe:0006:1-11.3", index: null, type: "image", path: "/home/tester/Pictures/sample-display.png", fps: null, rgb: null, orientation: 0 };
const managedPath = `/var/lib/lianli/media/imports/${"a".repeat(32)}/${"b".repeat(64)}.png`;

test("recognizes only import paths inside daemon state storage", () => {
  assert.equal(isManagedImportPath(managedPath, daemon.config_path), true);
  assert.equal(isManagedImportPath(lcd.path, daemon.config_path), false);
  assert.equal(isManagedImportPath(`/var/lib/other/media/imports/${"a".repeat(32)}/x.png`, daemon.config_path), false);
  assert.equal(isManagedImportPath(`/var/lib/lianli/media/imports/${"a".repeat(32)}/../x.png`, daemon.config_path), false);
});

test("reuses an already managed LCD path without starting another copy", async () => {
  const alreadyManaged = { ...lcd, path: managedPath };
  let starts = 0;
  const result = await copyLcdMediaToManaged(alreadyManaged, daemon, {
    async start() { starts++; throw new Error("must not create a duplicate import"); },
    async status() { throw new Error("must not inspect old operations"); },
    async result() { throw new Error("must not retrieve an import"); },
  }, () => {});
  assert.equal(starts, 0);
  assert.equal(result.path, managedPath);
});

test("offers Save for an unchanged saved LCD whose daemon media preparation failed", () => {
  assert.equal(shouldOfferLcdSave(lcd, false, "failed"), true);
  assert.equal(shouldOfferLcdSave({ ...lcd, path: managedPath }, false, "failed"), true);
  assert.equal(shouldOfferLcdSave(lcd, true, "ready"), true);
  assert.equal(shouldOfferLcdSave(lcd, false, "ready"), false);
  assert.equal(shouldOfferLcdSave({ ...lcd, type: "sensor" }, false, "failed"), false);
});

test("Save imports one selected LCD, waits for publication, and returns its managed reference", async () => {
  let importedInput;
  let statusCalls = 0;
  let resultCalls = 0;
  const phases = [];
  const result = await copyLcdMediaToManaged(lcd, daemon, {
    async start(instance, lcds, templates) {
      assert.equal(instance, daemon.instance_id);
      importedInput = lcds[0];
      assert.deepEqual(templates, []);
      return "a".repeat(32);
    },
    async status() {
      statusCalls++;
      return statusCalls === 1
        ? { id: "a".repeat(32), active: true, result: null, error: null }
        : { id: "a".repeat(32), active: false, result: { import_id: "a".repeat(32), lcds: [{ ...lcd, path: managedPath }], templates: [] }, error: null };
    },
    async result(instance, id) {
      resultCalls++;
      return { import_id: id, lcds: [{ ...lcd, path: managedPath }], templates: [] };
    },
    async verify(_instance, lcds) { return lcds[0].path === managedPath; },
  }, (phase) => phases.push(phase), { intervalMs: 0, wait: async () => {} });

  assert.equal(importedInput.path, lcd.path);
  assert.equal(lcd.path, "/home/tester/Pictures/sample-display.png");
  assert.equal(result.path, managedPath);
  assert.equal(statusCalls, 2);
  assert.equal(resultCalls, 1);
  assert.deepEqual(phases, ["authorizing", "copying"]);
});

test("new and previously saved failed selections both migrate to daemon-managed paths", async () => {
  const scenarios = [
    { name: "new selection", source: "/home/tester/Pictures/new-panel.png", dirty: true, state: "ready", omitIndex: true },
    { name: "existing saved private path", source: "/home/tester/Pictures/sample-display.png", dirty: false, state: "failed" },
  ];
  for (const [index, scenario] of scenarios.entries()) {
    const selected = { ...lcd, path: scenario.source };
    if (scenario.omitIndex) delete selected.index;
    assert.equal(shouldOfferLcdSave(selected, scenario.dirty, scenario.state), true, scenario.name);
    const id = String(index + 1).padStart(32, "0");
    const path = `/var/lib/lianli/media/imports/${id}/${String(index + 1).padStart(64, "a")}.png`;
    const imported = { ...selected, path, index: null };
    const migrated = await copyLcdMediaToManaged(selected, daemon, {
      async start() { return id; },
      async status() { return { id, active: false, result: { import_id: id, lcds: [imported], templates: [] }, error: null }; },
      async result() { return { import_id: id, lcds: [imported], templates: [] }; },
      async verify(_instance, lcds) { return lcds[0].path === path; },
    }, () => {});
    assert.equal(isManagedImportPath(migrated.path, daemon.config_path), true, scenario.name);
    assert.equal(selected.path, scenario.source, "migration must leave the original path unchanged");
  }
});

test("reuses a prior import only when its content hash matches and the daemon confirms access", async () => {
  const sourceBytes = new TextEncoder().encode("existing private LCD image").buffer;
  const hash = await sha256Hex(sourceBytes);
  const previousId = "c".repeat(32);
  const priorPath = `/var/lib/lianli/media/imports/${previousId}/${hash}.png`;
  let starts = 0;
  let verifiedPath = "";
  const reused = await copyLcdMediaToManaged(lcd, daemon, {
    async start() { starts++; throw new Error("must reuse valid prior copy"); },
    async status() {
      return {
        id: previousId, active: false, error: null,
        result: { import_id: previousId, lcds: [{ ...lcd, path: priorPath }], templates: [] },
      };
    },
    async result() {
      return { import_id: previousId, lcds: [{ ...lcd, path: priorPath }], templates: [] };
    },
    async verify(_instance, lcds) {
      verifiedPath = lcds[0].path;
      return true;
    },
  }, () => {}, { sourceSha256: hash });

  assert.equal(starts, 0);
  assert.equal(verifiedPath, priorPath);
  assert.equal(reused.path, priorPath);
});

test("does not reuse an inaccessible or different-content prior import", async () => {
  const priorId = "d".repeat(32);
  let starts = 0;
  const newId = "e".repeat(32);
  const newPath = `/var/lib/lianli/media/imports/${newId}/${"f".repeat(64)}.png`;
  const migrated = await copyLcdMediaToManaged(lcd, daemon, {
    async start() { starts++; return newId; },
    async status() {
      if (starts) {
        return {
          id: newId, active: false, error: null,
          result: { import_id: newId, lcds: [{ ...lcd, path: newPath }], templates: [] },
        };
      }
      return {
        id: priorId, active: false, error: null,
        result: { import_id: priorId, lcds: [{ ...lcd, path: `/var/lib/lianli/media/imports/${priorId}/${"0".repeat(64)}.png` }], templates: [] },
      };
    },
    async result(_instance, id) {
      if (id === priorId) return { import_id: priorId, lcds: [{ ...lcd, path: `/var/lib/lianli/media/imports/${priorId}/${"0".repeat(64)}.png` }], templates: [] };
      return { import_id: newId, lcds: [{ ...lcd, path: newPath }], templates: [] };
    },
    async verify(_instance, lcds) { return lcds[0].path === newPath; },
  }, () => {}, { sourceSha256: "1".repeat(64), intervalMs: 0, wait: async () => {} });

  assert.equal(starts, 1);
  assert.equal(migrated.path, newPath);
});

test("does not return success or permit persistence when a fresh managed copy is unreadable", async () => {
  const id = "9".repeat(32);
  const path = `/var/lib/lianli/media/imports/${id}/${"8".repeat(64)}.png`;
  await assert.rejects(copyLcdMediaToManaged(lcd, daemon, {
    async start() { return id; },
    async status() { return { id, active: false, error: null, result: { import_id: id, lcds: [], templates: [] } }; },
    async result() { return { import_id: id, lcds: [{ ...lcd, path }], templates: [] }; },
    async verify() { return false; },
  }, () => {}, { intervalMs: 0, wait: async () => {} }), /system daemon cannot read/i);
});

test("failed and mismatched imports remain recoverable errors", async () => {
  let started = false;
  await assert.rejects(copyLcdMediaToManaged({ ...lcd, path: null }, daemon, {
    async start() { started = true; return "a".repeat(32); },
    async status() { throw new Error("must not poll"); },
    async result() { throw new Error("must not fetch result"); },
  }, () => {}), /choose a media file/i);
  assert.equal(started, false);

  await assert.rejects(copyLcdMediaToManaged(lcd, daemon, {
    async start() { return "a".repeat(32); },
    async status() { return { id: "a".repeat(32), active: false, result: null, error: "permission denied" }; },
    async result() { throw new Error("must not fetch result"); },
  }, () => {}, { intervalMs: 0, wait: async () => {} }), /permission denied/);

  await assert.rejects(copyLcdMediaToManaged(lcd, daemon, {
    async start() { return "a".repeat(32); },
    async status() { return { id: "a".repeat(32), active: false, result: { import_id: "a".repeat(32), lcds: [], templates: [] }, error: null }; },
    async result() { return { import_id: "a".repeat(32), lcds: [{ ...lcd, path: "/home/tester/Pictures/sample-display.png" }], templates: [] }; },
  }, () => {}, { intervalMs: 0, wait: async () => {} }), /managed copy/);
});
