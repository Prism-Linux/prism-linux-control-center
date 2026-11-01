import test from "node:test";
import assert from "node:assert/strict";
import { isProxy, reactive } from "vue";
import { serializeInvokeArgs, toPlainData } from "../src/utils/plainData.ts";
import { copyLcdMediaToManaged } from "../src/utils/managedMediaImport.ts";

test("reproduces structuredClone DataCloneError on the reactive LCD Save input", () => {
  const lcd = reactive({
    serial: "hid:1cbe:0006:1-11.3",
    index: null,
    type: "image",
    path: "/home/tester/Pictures/sample-display.png",
    fps: null,
    rgb: null,
    orientation: 90,
    brightness: 72,
    framing: reactive({ fit: "cover", zoom: 1.35, offset_x: 0.1, offset_y: -0.2 }),
  });

  let thrown;
  try {
    structuredClone(lcd);
  } catch (error) {
    thrown = error;
  }
  assert.equal(thrown?.name, "DataCloneError");
  assert.match(thrown.stack, /structuredClone/);
});

test("normalizes nested reactive config and profiles to plain IPC-safe data", () => {
  const config = reactive({
    lcds: [reactive({
      serial: "hid:1cbe:0006:1-11.3",
      type: "image",
      path: "/var/lib/lianli/media/imports/abc/image.png",
      orientation: 90,
      brightness: 72,
      framing: reactive({ fit: "cover", zoom: 1.35, offset_x: 0.1, offset_y: -0.2 }),
    })],
    lcd_profiles: [reactive({
      id: "profile-1",
      name: "Desk",
      device_id: "hid:1cbe:0006:1-11.3",
      lcd: reactive({ type: "image", path: "/managed/profile.png", orientation: 180 }),
    })],
    unrelated: reactive({ retained: true }),
  });

  const payload = serializeInvokeArgs({ method: "SetConfig", params: { config } });
  assert.equal(isProxy(payload), false);
  assert.equal(isProxy(payload.params.config.lcds[0]), false);
  assert.equal(isProxy(payload.params.config.lcds[0].framing), false);
  assert.equal(payload.params.config.lcds[0].orientation, 90);
  assert.equal(payload.params.config.lcds[0].brightness, 72);
  assert.equal(payload.params.config.lcds[0].framing.zoom, 1.35);
  assert.equal(payload.params.config.lcd_profiles[0].name, "Desk");
  assert.equal(payload.params.config.unrelated.retained, true);
  assert.doesNotThrow(() => structuredClone(payload));
});

test("managed import receives a plain LCD payload and persists its settings", async () => {
  const lcd = reactive({
    serial: "hid:1cbe:0006:1-11.3",
    index: null,
    type: "image",
    path: "/home/tester/Pictures/sample-display.png",
    fps: null,
    rgb: null,
    orientation: 90,
    brightness: 72,
    framing: reactive({ fit: "cover", zoom: 1.35, offset_x: 0.1, offset_y: -0.2 }),
  });
  const configPath = "/var/lib/lianli/config.json";
  const importedPath = `/var/lib/lianli/media/imports/${"a".repeat(32)}/${"b".repeat(64)}.png`;
  let serializedSelection;

  const result = await copyLcdMediaToManaged(lcd, {
    instance_id: "system-daemon",
    config_path: configPath,
  }, {
    async start(_instance, lcds) {
      serializedSelection = lcds[0];
      assert.equal(isProxy(lcds[0]), false);
      assert.equal(isProxy(lcds[0].framing), false);
      return "a".repeat(32);
    },
    async status() {
      return { id: "a".repeat(32), active: false, error: null, result: { import_id: "a".repeat(32), lcds: [], templates: [] } };
    },
    async result() {
      return {
        import_id: "a".repeat(32),
        lcds: [{ ...serializedSelection, path: importedPath }],
        templates: [],
      };
    },
    async verify(_instance, lcds) { return lcds[0].path === importedPath; },
  }, () => {});

  assert.equal(result.path, importedPath);
  assert.equal(result.orientation, 90);
  assert.equal(result.brightness, 72);
  assert.deepEqual(result.framing, { fit: "cover", zoom: 1.35, offset_x: 0.1, offset_y: -0.2 });
  assert.equal(lcd.path, "/home/tester/Pictures/sample-display.png");
});

test("rejects browser objects instead of silently sending empty placeholders", () => {
  const file = new File(["data"], "image.png", { type: "image/png" });
  assert.throws(() => toPlainData({ config: { media: file } }), /Unsupported object.*media/);
  assert.throws(() => toPlainData({ config: { callback() {} } }), /Non-serializable value.*callback/);
});
