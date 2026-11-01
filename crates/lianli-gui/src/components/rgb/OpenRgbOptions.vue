<script setup lang="ts">
import { computed } from "vue";
import { useConfigStore } from "@/stores/config";
import { useTranslate, type MessageKey } from "@/i18n";
import type { RgbDeviceCapabilities } from "@/types";

type OptionKey = "openrgb_regions" | "openrgb_per_fan_zones";

interface Option {
  key: OptionKey;
  label: MessageKey;
  hint: MessageKey;
  appliesTo: (cap: RgbDeviceCapabilities) => boolean;
}

const OPTIONS: Option[] = [
  {
    key: "openrgb_regions",
    label: "rgb.openrgbRegions.label",
    hint: "rgb.openrgbRegions.hint",
    appliesTo: (cap) => /^hid:0cf2:a104(:|$)/.test(cap.device_id) && !!cap.hardware_group_effects,
  },
  {
    key: "openrgb_per_fan_zones",
    label: "rgb.openrgbFanZones.label",
    hint: "rgb.openrgbFanZones.hint",
    // SL V4 groups always keep per-fan zones in the daemon.
    appliesTo: (cap) =>
      cap.device_id.startsWith("wireless:") && cap.sync_led_count != null && cap.render_profile?.family !== "SlV4",
  },
];

const t = useTranslate();
const config = useConfigStore();

function checked(key: OptionKey): boolean {
  return config.ensureRgb()[key] ?? false;
}

function setChecked(key: OptionKey, value: boolean) {
  config.ensureRgb()[key] = value;
  config.markDirty();
}

// Options stay visible while enabled so they can be turned off with the device offline.
const visible = computed(() =>
  OPTIONS.filter((option) => checked(option.key) || config.rgbCaps.some(option.appliesTo)),
);
</script>

<template>
  <div v-if="visible.length" class="openrgb-options">
    <div class="options">
      <div v-for="option in visible" :key="option.key" class="option">
        <n-checkbox :checked="checked(option.key)" @update:checked="(value: boolean) => setChecked(option.key, value)">
          {{ t(option.label) }}
        </n-checkbox>
        <span class="hint">{{ t(option.hint) }}</span>
      </div>
    </div>
    <p class="hint note">{{ t("rgb.openrgbOptions.reconnectNote") }}</p>
  </div>
</template>

<style scoped>
.openrgb-options {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.options {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2) var(--space-4);
}
.option {
  flex: 1 1 260px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.option .hint {
  padding-left: 24px;
}
.note {
  margin: 0;
}
</style>
