import test from "node:test";
import assert from "node:assert/strict";
import { finitePercent, linePath } from "../src/utils/visualization.ts";

test("live trend paths only render finite samples", () => {
  assert.equal(linePath([null, Number.NaN, Infinity]), "");
  assert.equal(linePath([]), "");
  assert.match(linePath([null, 1200]), /^M \d+\.\d+ \d+\.\d+$/);
  assert.match(linePath([1, 3, 2]), /^M .+ L .+ L .+$/);
});

test("dashboard usage values reject invalid percentages", () => {
  assert.equal(finitePercent(0), 0);
  assert.equal(finitePercent(100), 100);
  assert.equal(finitePercent(-1), null);
  assert.equal(finitePercent(101), null);
  assert.equal(finitePercent(Number.NaN), null);
});
