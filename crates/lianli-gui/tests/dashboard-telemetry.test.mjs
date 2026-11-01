import test from "node:test";
import assert from "node:assert/strict";
import { liveCoolantTemperature, liveMemoryUsage, liveTemperature, meanFanRpm } from "../src/utils/dashboardTelemetry.ts";
import { motherboardSyncEnabled, quickSyncControls } from "../src/components/rgb/syncSettings.ts";

const sensor = (name, value, unit = "C") => ({
  source: { type: "hwmon", name: "test", label: name },
  sensor_name: null,
  display_name: name,
  divider: 1,
  unit,
  current_value: value,
});

test("dashboard reports only recent finite CPU and GPU temperature readings", () => {
  const sensors = [sensor("CPU Package", 63.4), sensor("GPU Temperature", 51.6), sensor("CPU Usage", 94, "PERCENT")];
  assert.equal(liveTemperature(sensors, "cpu", 20_000, 21_000), 63.4);
  assert.equal(liveTemperature(sensors, "gpu", 20_000, 21_000), 51.6);
  assert.equal(liveTemperature(sensors, "cpu", 1_000, 21_000), null);
  assert.equal(liveTemperature(sensors, "gpu", null, 21_000), null);
});

test("dashboard selects CPU package readings from hwmon identity when display_name is absent", () => {
  const core = { ...sensor("temp2", 31), display_name: null, sensor_name: { device_name: "CPU", sensor_name: "Temp Core 0" } };
  const packageReading = { ...sensor("temp1", 34), display_name: null, sensor_name: { device_name: "CPU", sensor_name: "Control Temp" } };
  const nvme = { ...sensor("Composite", 44), display_name: null, source: { type: "hwmon", name: "nvme", label: "temp1" }, sensor_name: { device_name: "NVMe SSD", sensor_name: "Composite" } };
  assert.equal(liveTemperature([nvme, core, packageReading], "cpu", 20_000, 20_000), 34);
});

test("dashboard uses fresh daemon host memory usage and validates its range", () => {
  const ram = { ...sensor("RAM: Usage", 31.2, "PERCENT"), source: { type: "mem_usage" } };
  assert.equal(liveMemoryUsage([ram], 20_000, 21_000), 31.2);
  assert.equal(liveMemoryUsage([{ ...ram, current_value: 0 }], 20_000, 21_000), 0);
  assert.equal(liveMemoryUsage([{ ...ram, current_value: 143 }], 20_000, 21_000), null);
  assert.equal(liveMemoryUsage([ram], 1_000, 20_000), null);
});

test("coolant reading uses only the selected hwmon source, Celsius value, and a fresh sample", () => {
  const source = { type: "hwmon", name: "asus-ec-sensors", label: "temp1", device_path: "platform:asus-ec-sensors" };
  const coolant = { ...sensor("T_Sensor", 32.75), source, sensor_name: { device_name: "ASUS EC", sensor_name: "T_Sensor" } };
  const cpu = sensor("CPU Package", 81);
  assert.equal(liveCoolantTemperature([cpu, coolant], source, 20_000, 20_100), 32.75);
  assert.equal(liveCoolantTemperature([cpu, coolant], { ...source, label: "temp2" }, 20_000, 20_100), null);
  assert.equal(liveCoolantTemperature([{ ...coolant, current_value: null }], source, 20_000, 20_100), null);
  assert.equal(liveCoolantTemperature([{ ...coolant, current_value: 240 }], source, 20_000, 20_100), null);
  assert.equal(liveCoolantTemperature([coolant], source, 1_000, 20_000), null);
  assert.equal(liveCoolantTemperature([coolant], null, 20_000, 20_100), null);
});

test("dashboard leaves unsupported and invalid sensor values unavailable", () => {
  assert.equal(liveTemperature([sensor("CPU Package", null)], "cpu", 20_000, 20_000), null);
  assert.equal(liveTemperature([sensor("CPU Package", Number.NaN)], "cpu", 20_000, 20_000), null);
  assert.equal(liveTemperature([sensor("GPU Usage", 80, "PERCENT")], "gpu", 20_000, 20_000), null);
});

test("fan average ignores stopped and invalid RPM values", () => {
  assert.equal(meanFanRpm([1200, 1400, 0, Number.NaN, -1]), 1300);
  assert.equal(meanFanRpm([0, Number.NaN]), null);
});

test("motherboard ARGB ownership remains independent of Quick Sync state", () => {
  const cap = { device_id: "controller:port0:group0", hardware_group_effects: false, sync_led_count: 16, supported_modes: ["Rainbow"] };
  const rgb = {
    devices: [{ device_id: cap.device_id, mb_rgb_sync: true }],
    merge_lighting: { enabled: true, device_order: [cap.device_id], disabled_devices: [], kind: "Matched", effect: { mode: "Rainbow" } },
  };
  assert.equal(motherboardSyncEnabled(cap, rgb), true);
  assert.equal(quickSyncControls(cap, rgb), false);
  rgb.devices[0].mb_rgb_sync = false;
  assert.equal(motherboardSyncEnabled(cap, rgb), false);
  assert.equal(quickSyncControls(cap, rgb), true);
});
