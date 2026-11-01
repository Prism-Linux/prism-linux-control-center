<script setup lang="ts">
import { computed, ref } from "vue";
import { ChevronDown, ChevronRight, Fan } from "lucide-vue-next";
import type { DeviceInfo, FanGroup, FanSpeed, PwmHeader } from "@/types";
import { MB_SYNC_KEY, MB_SYNC_PREFIX } from "@/types";
import { decodeSpeedMode, fillSlots, isMbSyncGroup, speedModeOf } from "./speedMode";
import { useConfigStore } from "@/stores/config";
import { useDevicesStore } from "@/stores/devices";
import { useDeviceNamesStore } from "@/stores/deviceNames";
import LabeledSlider from "@/components/common/LabeledSlider.vue";
import { FAMILY_DISPLAY } from "@/constants";
import { useTranslate } from "@/i18n";

const props = defineProps<{
  device: DeviceInfo;
  group: FanGroup;
  curveNames: string[];
  pwmHeaders: PwmHeader[];
}>();

const config = useConfigStore();
const t = useTranslate();

const expanded = ref(true);

const devices = useDevicesStore();
const names = useDeviceNamesStore();

const title = computed(() =>
  names.label(props.device.device_id, devices.labels.get(props.device.device_id) ?? props.device.name),
);
const family = computed(() => {
  const fam = FAMILY_DISPLAY[props.device.family] ?? props.device.family;
  return fam === props.device.name ? null : fam;
});

const rpms = computed(() => devices.fanRpms(props.device.device_id));
const visibleSlots = computed(() => Math.min(slotCount.value, 4));

const commonMode = computed(() => {
  const first = modeOf(0);
  for (let slot = 1; slot < visibleSlots.value; slot++) {
    if (modeOf(slot) !== first) return null;
  }
  return first;
});

function onAllModes(value: string) {
  if (value === MB_SYNC_KEY) {
    onMode(0, value);
    return;
  }
  setAllSlots(decodeSpeedMode(value));
}

const perFan = computed(() => props.device.per_fan_control ?? false);
const slotCount = computed(() => props.device.fan_count ?? 1);

function speedAt(slot: number): FanSpeed {
  return props.group.speeds[slot];
}

function setSpeed(slot: number, value: FanSpeed) {
  const next = [...props.group.speeds] as FanSpeed[];
  next[slot] = value;
  props.group.speeds = next as [FanSpeed, FanSpeed, FanSpeed, FanSpeed];
  config.markDirty();
}

// MB Sync is a port-wide hardware setting, not per-fan: selecting it applies
// to all slots, and leaving it clears it from the whole port.
function groupIsMbSync(): boolean {
  return isMbSyncGroup(props.group.speeds);
}
function setAllSlots(value: FanSpeed) {
  fillSlots(props.group, value);
  config.markDirty();
}

// Speed mode dropdown options: Off / curve names / Constant PWM / MB Sync.
const modeOptions = computed(() => {
  const opts: { label: string; value: string; disabled?: boolean }[] = [
    { label: t("common.off"), value: "off" },
    ...props.curveNames.map((n) => ({ label: t("fans.speedMode.curve", { name: n }), value: `curve:${n}` })),
    { label: t("fans.speedMode.constantPwm"), value: "constant" },
    {
      label: t("fans.speedMode.mbSync"),
      value: MB_SYNC_KEY,
      disabled: !props.device.mb_sync_support && !props.pwmHeaders.length,
    },
  ];
  return opts;
});

// PWM header options for the MB Sync source dropdown.
const pwmHeaderOptions = computed(() => {
  if (!props.pwmHeaders.length) return [];
  return props.pwmHeaders.map((h) => ({ label: h.label, value: h.id }));
});

function modeOf(slot: number): string {
  return speedModeOf(props.group.speeds, slot);
}

function onMode(slot: number, value: string) {
  if (value === MB_SYNC_KEY) {
    if (props.device.mb_sync_support) {
      setAllSlots(MB_SYNC_KEY);
    } else {
      const source = currentPwmSource.value || props.pwmHeaders[0]?.id;
      if (source) setAllSlots(`${MB_SYNC_PREFIX}${source}`);
    }
    return;
  }
  const decoded = decodeSpeedMode(value);
  if (groupIsMbSync()) {
    // Leaving MB Sync is also port-wide: reset the whole port to a constant
    // default, then apply the chosen mode to this fan.
    const next: FanSpeed[] = [128, 128, 128, 128];
    next[slot] = decoded;
    props.group.speeds = next as [FanSpeed, FanSpeed, FanSpeed, FanSpeed];
    config.markDirty();
  } else {
    setSpeed(slot, decoded);
  }
}

// ── PWM source selection (MB Sync) ──────────────────────────────────────────
// Returns the header id from the first slot that has one, or empty string.
const currentPwmSource = computed(() => {
  for (const s of props.group.speeds) {
    if (typeof s === "string" && s.startsWith(MB_SYNC_PREFIX)) {
      return s.slice(MB_SYNC_PREFIX.length);
    }
  }
  return "";
});

function onPwmSource(headerId: string | null) {
  if (!headerId) return;
  const value = `${MB_SYNC_PREFIX}${headerId}`;
  setAllSlots(value);
}

function pwmOf(slot: number): number {
  const s = speedAt(slot);
  return typeof s === "number" ? s : 128;
}
function setPwm(slot: number, v: number) {
  setSpeed(slot, v);
}
</script>

<template>
  <div class="fan-group">
    <div class="head">
      <button class="toggle" @click="expanded = !expanded">
        <component :is="expanded ? ChevronDown : ChevronRight" :size="16" />
        <span class="title">{{ title }}</span>
        <span v-if="family" class="muted">{{ family }}</span>
        <span class="muted">· {{ t(perFan ? "fans.groupCard.fanCount" : "fans.groupCard.portCount", { count: visibleSlots }) }}</span>
      </button>
      <div v-if="visibleSlots > 1" class="all">
        <span class="muted">{{ t("fans.groupCard.all") }}</span>
        <n-select
          size="small"
          :value="commonMode"
          :options="modeOptions"
          :placeholder="t('fans.groupCard.mixed')"
          @update:value="onAllModes"
        />
      </div>
    </div>

    <div v-if="expanded" class="slots">
      <div v-for="slot in visibleSlots" :key="slot - 1" class="slot">
        <span class="slot-label">{{ t(perFan ? "fans.groupCard.fanSlot" : "fans.groupCard.portSlot", { index: slot }) }}</span>
        <n-select
          class="slot-mode"
          size="small"
          :value="modeOf(slot - 1)"
          :options="modeOptions"
          @update:value="(v: string) => onMode(slot - 1, v)"
        />
        <div class="slot-pwm">
          <LabeledSlider
            v-if="modeOf(slot - 1) === 'constant'"
            :model-value="pwmOf(slot - 1)"
            :min="0"
            :max="255"
            :step="1"
            :format="(v: number) => `${Math.round((v / 255) * 100)}%`"
            @update:model-value="(v: number) => setPwm(slot - 1, Math.round(v))"
          />
        </div>
        <span class="rpm" :title="t('fans.groupCard.fanSpeedTooltip', { index: slot })">
          <Fan :size="12" />
          <span class="mono">{{ rpms[slot - 1] != null ? `${rpms[slot - 1]} rpm` : "—" }}</span>
        </span>
      </div>
    </div>

    <div v-if="groupIsMbSync() && !device.mb_sync_support" class="pwm-source-row">
      <label class="muted">{{ t("fans.groupCard.pwmSource") }}</label>
      <n-select
        size="small"
        :value="currentPwmSource"
        :options="pwmHeaderOptions"
        :placeholder="t('fans.groupCard.pwmSourcePlaceholder')"
        @update:value="onPwmSource"
      />
      <span class="hint">{{ t("fans.groupCard.pwmSourceHint") }}</span>
    </div>
  </div>
</template>

<style scoped>
.fan-group {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  padding: var(--space-2) 0;
}
.fan-group + .fan-group {
  border-top: 1px solid var(--border);
}
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
}
.toggle {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
  padding: 0;
  border: none;
  background: none;
  color: var(--text-primary);
  font: inherit;
  cursor: pointer;
}
.title {
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.muted {
  font-size: var(--font-size-sm);
  white-space: nowrap;
}
.all {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  width: 240px;
  flex-shrink: 0;
}
.slots {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  padding-left: 24px;
}
.slot {
  display: grid;
  grid-template-columns: 56px 200px minmax(0, 1fr) 90px;
  align-items: center;
  gap: var(--space-3);
  min-height: 32px;
}
.slot-label {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
}
.rpm {
  display: inline-flex;
  align-items: center;
  justify-content: flex-end;
  gap: 4px;
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
}
.mono {
  font-family: var(--font-mono);
  color: var(--text-primary);
}
.pwm-source-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-2);
  padding-left: 24px;
}
.pwm-source-row .n-select {
  width: 260px;
}
@media (max-width: 720px) {
  .slot {
    grid-template-columns: 56px minmax(0, 1fr) 80px;
  }
  .slot-pwm {
    grid-column: 2 / -1;
  }
}
</style>
