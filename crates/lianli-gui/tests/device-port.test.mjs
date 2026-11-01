import test from "node:test";
import assert from "node:assert/strict";
import { enePortIndex } from "../src/utils/devicePort.ts";

test("device port labels follow the hardware id or reported device name", () => {
  assert.equal(enePortIndex("hid:0cf2:a102:1-10.1:port3", "ENE 6K77 SL/AL Fan Controller"), 3);
  assert.equal(enePortIndex("hid:controller", "ENE 6K77 SL/AL Fan Controller Port 2"), 2);
  assert.equal(enePortIndex("hid:controller", "TLV2 Wireless LCD Fan"), null);
  assert.equal(enePortIndex("hid:controller:port-1", "ENE 6K77 Controller"), null);
});
