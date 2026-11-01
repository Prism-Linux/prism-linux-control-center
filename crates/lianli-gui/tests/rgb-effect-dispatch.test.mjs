import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

test("editing an RGB zone dispatches a live effect update to the daemon store", async () => {
  const source = await readFile(new URL("../src/components/rgb/RgbZoneEditor.vue", import.meta.url), "utf8");
  const patchEffect = source.split("function patchEffect(")[1]?.split("\nfunction modeOptions()")[0] ?? "";

  assert.match(patchEffect, /rgb\.scheduleEffect\(props\.deviceId, props\.zoneIndex,/);
  assert.match(patchEffect, /colors:\s*effect\.value\.colors\.map\(\(color\)\s*=>\s*\[\.\.\.color\]/);
});
