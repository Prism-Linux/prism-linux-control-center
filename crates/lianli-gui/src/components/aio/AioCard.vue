<script setup lang="ts">
import { computed } from "vue";
import type { DeviceInfo, AioConfig, FanSpeed } from "@/types";
import { useConfigStore } from "@/stores/config";
import { useDevicesStore } from "@/stores/devices";
import { useFansStore } from "@/stores/fans";
import { useDeviceNamesStore } from "@/stores/deviceNames";
import ColorPicker from "@/components/rgb/ColorPicker.vue";
import LabeledSlider from "@/components/common/LabeledSlider.vue";
import SensorSelect from "@/components/common/SensorSelect.vue";
import { enumerateSensorsAsOptions, optionForConfig, decodeOption } from "@/stores/sensorOptions";
import { useTranslate } from "@/i18n";

const t = useTranslate();
const props = defineProps<{ device: DeviceInfo }>();

const config = useConfigStore();
const devices = useDevicesStore();
const fans = useFansStore();
const names = useDeviceNamesStore();

const aio = computed<AioConfig>(() => config.aioConfigFor(props.device.device_id));
const pumpRange = computed(() => props.device.pump_rpm_range ?? [0, 4200]);
const fanCount = computed(() => props.device.fan_count ?? 0);
const hasPump = computed(() => props.device.has_pump ?? false);
const hasFan = computed(() => props.device.has_fan ?? false);

const pumpRpm = computed(() => {
  const rpms = devices.fanRpms(props.device.device_id);
  return rpms[fanCount.value] ?? null;
});
const fanRpms = computed(() => {
  const rpms = devices.fanRpms(props.device.device_id);
  return rpms.slice(0, fanCount.value);
});
const coolant = computed(() => devices.coolantTemp(props.device.device_id));

const pumpMode = computed(() => speedMode(aio.value.pump_target_rpm));
const pumpOptions = computed(() => buildSpeedOptions(Boolean(props.device.pump_mb_sync_support)));
const fanOptions = computed(() => buildSpeedOptions(props.device.mb_sync_support));

function buildSpeedOptions(mbSync: boolean) {
  const curves = config.config.fan_curves.map((c) => ({
    label: t("devices.aio.curveOption", { name: c.name }),
    value: `curve:${c.name}`,
  }));
  return [
    { label: t("common.off"), value: "off" },
    ...curves,
    { label: t("devices.aio.constantPwm"), value: "constant" },
    ...(mbSync ? [{ label: t("devices.aio.mbSync"), value: "__mb_sync__" }] : []),
  ];
}

function speedMode(s: FanSpeed): string {
  if (typeof s === "number") return "constant";
  if (s === "__mb_sync__" || s.startsWith("__mb_sync__:")) return "__mb_sync__";
  if (s === "off" || s === "") return "off";
  return `curve:${s}`;
}

function onPumpMode(value: string) {
  aio.value.pump_target_rpm = decodeMode(value, aio.value.pump_target_rpm);
  config.markDirty();
}
function onFanMode(slot: number, value: string) {
  const next = [...aio.value.fan_speeds] as FanSpeed[];
  next[slot] = decodeMode(value, next[slot]);
  aio.value.fan_speeds = next;
  config.markDirty();
}
function decodeMode(value: string, current: FanSpeed): FanSpeed {
  if (value === "off") return "off";
  if (value === "constant") return typeof current === "number" ? current : 128;
  if (value === "__mb_sync__") return "__mb_sync__";
  if (value.startsWith("curve:")) return value.slice("curve:".length);
  return current;
}

function pumpPwm(): number {
  const s = aio.value.pump_target_rpm;
  return typeof s === "number" ? s : 128;
}
function setPumpPwm(v: number) {
  aio.value.pump_target_rpm = v;
  config.markDirty();
}
function fanPwm(slot: number): number {
  const s = aio.value.fan_speeds[slot];
  return typeof s === "number" ? s : 128;
}
function setFanPwm(slot: number, v: number) {
  const next = [...aio.value.fan_speeds] as FanSpeed[];
  next[slot] = v;
  aio.value.fan_speeds = next;
  config.markDirty();
}

// ── Sensor dropdowns ────────────────────────────────────────────────────────
const sensorOptions = computed(() => enumerateSensorsAsOptions(config.sensors, false));

function sensorValue(key: "cpu_temp_source" | "cpu_load_source" | "gpu_temp_source" | "gpu_load_source") {
  return optionForConfig(config.sensors, aio.value[key]);
}
function onSensor(key: "cpu_temp_source" | "cpu_load_source" | "gpu_temp_source" | "gpu_load_source", value: string) {
  aio.value[key] = decodeOption(value);
  config.markDirty();
}

// ── Display settings ────────────────────────────────────────────────────────
const rotationOptions = [
  { label: "0°", value: 0 },
  { label: "90°", value: 1 },
  { label: "180°", value: 2 },
  { label: "270°", value: 3 },
];
const themeOptions = computed(() => Array.from({ length: 13 }, (_, i) => ({
  label: t("devices.aio.themeOption", { index: i }),
  value: i,
})));

function setColor(key: "str_color" | "val_color" | "unit_color", v: any) {
  aio.value[key] = v;
  config.markDirty();
}

const mac = computed(() =>
  props.device.device_id.startsWith("wireless:")
    ? props.device.device_id.slice("wireless:".length)
    : props.device.device_id,
);
</script>

<template>
  <div class="card aio">
    <div class="head">
      <div>
        <div class="name">{{ names.label(device.device_id, device.name) }}</div>
        <div class="muted mono">{{ mac }}</div>
      </div>
      <div class="telemetry">
        <span v-for="(rpm, i) in fanRpms" :key="'fan' + i">{{ t("devices.aio.fanTelemetry", { index: i + 1, rpm }) }}</span>
        <span v-if="pumpRpm !== null">{{ t("devices.aio.pumpTelemetry", { rpm: pumpRpm }) }}</span>
        <span v-if="coolant !== null">{{ t("devices.aio.coolantTelemetry", { temp: coolant.toFixed(1) }) }}</span>
      </div>
    </div>

    <div class="grid">
      <!-- Pump -->
      <div class="field">
        <label class="muted">{{ t("devices.aio.pumpSpeed") }}</label>
        <n-select size="small" :value="pumpMode" :options="pumpOptions" :disabled="!hasPump" @update:value="onPumpMode" />
        <LabeledSlider
          v-if="pumpMode === 'constant'"
          :model-value="pumpPwm()"
          :min="0"
          :max="255"
          @update:model-value="setPumpPwm"
        />
        <span v-if="!hasPump" class="note">{{ t("devices.aio.controlUnavailable") }}</span>
        <span v-else-if="pumpMode === '__mb_sync__' && !device.pump_mb_sync_support" class="note">{{ t("devices.aio.pumpMbSyncUnavailable") }}</span>
      </div>

      <!-- Attached fans -->
      <div v-if="fanCount > 0" class="field" v-for="slot in Math.min(fanCount, 4)" :key="slot - 1">
        <label class="muted">{{ t("devices.aio.fan", { index: slot }) }}</label>
        <n-select
          size="small"
          :value="speedMode(aio.fan_speeds[slot - 1])"
          :options="fanOptions"
          :disabled="!hasFan"
          @update:value="(v: string) => onFanMode(slot - 1, v)"
        />
        <span v-if="speedMode(aio.fan_speeds[slot - 1]) === '__mb_sync__' && !device.mb_sync_support" class="note">{{ t("devices.aio.fanMbSyncUnavailable") }}</span>
        <LabeledSlider
          v-if="speedMode(aio.fan_speeds[slot - 1]) === 'constant'"
          :model-value="fanPwm(slot - 1)"
          :min="0"
          :max="255"
          @update:model-value="(v: number) => setFanPwm(slot - 1, v)"
        />
        <span v-if="!hasFan && slot === 1" class="note">{{ t("devices.aio.controlUnavailable") }}</span>
      </div>

      <!-- Display sensors -->
      <div class="field"><label class="muted">{{ t("devices.aio.cpuTemp") }}</label>
        <SensorSelect size="small" :value="sensorValue('cpu_temp_source')" :options="sensorOptions" @update:value="(v) => onSensor('cpu_temp_source', v)" filterable />
      </div>
      <div class="field"><label class="muted">{{ t("devices.aio.cpuLoad") }}</label>
        <SensorSelect size="small" :value="sensorValue('cpu_load_source')" :options="sensorOptions" @update:value="(v) => onSensor('cpu_load_source', v)" filterable />
      </div>
      <div class="field"><label class="muted">{{ t("devices.aio.gpuTemp") }}</label>
        <SensorSelect size="small" :value="sensorValue('gpu_temp_source')" :options="sensorOptions" @update:value="(v) => onSensor('gpu_temp_source', v)" filterable />
      </div>
      <div class="field"><label class="muted">{{ t("devices.aio.gpuLoad") }}</label>
        <SensorSelect size="small" :value="sensorValue('gpu_load_source')" :options="sensorOptions" @update:value="(v) => onSensor('gpu_load_source', v)" filterable />
      </div>

      <!-- Text colors -->
      <div class="field colors">
        <label class="muted">{{ t("devices.aio.labelColor") }}</label>
        <ColorPicker :model-value="aio.str_color" alpha @update:model-value="(v) => setColor('str_color', v)" />
      </div>
      <div class="field colors">
        <label class="muted">{{ t("devices.aio.valueColor") }}</label>
        <ColorPicker :model-value="aio.val_color" alpha @update:model-value="(v) => setColor('val_color', v)" />
      </div>
      <div class="field colors">
        <label class="muted">{{ t("devices.aio.unitColor") }}</label>
        <ColorPicker :model-value="aio.unit_color" alpha @update:model-value="(v) => setColor('unit_color', v)" />
      </div>

      <!-- Display -->
      <div class="field">
        <label class="muted">{{ t("devices.aio.brightness") }}</label>
        <LabeledSlider :model-value="aio.brightness" :min="0" :max="100" suffix="%" @update:model-value="(v: number) => { aio.brightness = v; config.markDirty(); }" />
      </div>
      <div class="field">
        <label class="muted">{{ t("devices.aio.rotation") }}</label>
        <n-select v-model:value="aio.rotation" size="small" :options="rotationOptions" @update:value="() => config.markDirty()" />
      </div>
      <div class="field">
        <label class="muted">{{ t("devices.aio.theme") }}</label>
        <n-select v-model:value="aio.theme_index" size="small" :options="themeOptions" @update:value="() => config.markDirty()" />
      </div>
      <div class="field">
        <label class="muted">{{ t("devices.aio.loopInterval") }}</label>
        <n-input-number v-model:value="aio.loop_interval" size="small" :min="1" :max="30" @update:value="() => config.markDirty()" />
      </div>
    </div>
  </div>
</template>

<style scoped>
.aio {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}
.note {
  color: var(--text-secondary);
  font-size: var(--font-size-xs);
}
.head {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
}
.name {
  font-weight: 600;
}
.mono {
  font-family: var(--font-mono);
  font-size: var(--font-size-xs);
}
.telemetry {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  font-variant-numeric: tabular-nums;
}
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: var(--space-3);
}
.field {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}
</style>
