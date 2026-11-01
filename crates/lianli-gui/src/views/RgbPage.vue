<script setup lang="ts">
import { computed, ref } from "vue";
import { Layers, Palette, Wifi } from "lucide-vue-next";
import type { RgbDeviceCapabilities } from "@/types";
import { useDaemonStore } from "@/stores/daemon";
import { useConfigStore } from "@/stores/config";
import { useDeviceNamesStore } from "@/stores/deviceNames";
import { useDevicesStore } from "@/stores/devices";
import { rgbDeviceLabels } from "@/utils/deviceNames";
import RgbDeviceCard from "@/components/rgb/RgbDeviceCard.vue";
import RgbSyncEditor from "@/components/rgb/RgbSyncEditor.vue";
import AsusAuraControls from "@/components/rgb/AsusAuraControls.vue";
import OpenRgbOptions from "@/components/rgb/OpenRgbOptions.vue";
import StatusDot from "@/components/common/StatusDot.vue";
import { motherboardSyncEnabled, quickSyncControls } from "@/components/rgb/syncSettings";
import { useTranslate } from "@/i18n";
import { backendText } from "@/utils/backendText";

const QUICK_SYNC = "quick-sync";

const t = useTranslate();
const daemon = useDaemonStore();
const config = useConfigStore();
const names = useDeviceNamesStore();
const devices = useDevicesStore();

const rgb = computed(() => config.ensureRgb());

const openrgbEnabled = computed({
  get: () => rgb.value.openrgb_server,
  set: (v: boolean) => {
    rgb.value.openrgb_server = v;
    void config.save();
  },
});

const openrgbPort = computed({
  get: () => rgb.value.openrgb_port,
  set: (v: number) => {
    rgb.value.openrgb_port = v;
    config.markDirty();
  },
});

const statusText = computed(() => {
  if (!openrgbEnabled.value) return t("common.disabled");
  if (daemon.openrgbError) return t("rgb.page.openrgbError", { error: backendText(daemon.openrgbError, daemon.openrgbErrorUi) });
  if (daemon.openrgbRunning) return t("rgb.page.openrgbListening", { port: daemon.openrgbPort ?? rgb.value.openrgb_port });
  return t("rgb.page.openrgbStarting");
});

const rgbCaps = computed(() => config.rgbCaps.filter((c) => !c.rf_owned));
const labels = computed(() => rgbDeviceLabels(rgbCaps.value, devices.labels));
function defaultLabel(cap: RgbDeviceCapabilities): string {
  return labels.value.get(cap.device_id) ?? cap.device_name;
}

const syncedCount = computed(() => rgbCaps.value.filter((cap) => quickSyncControls(cap, rgb.value)).length);
const motherboardSyncedCount = computed(() => rgbCaps.value.filter((cap) => motherboardSyncEnabled(cap, rgb.value)).length);
const syncSummary = computed(() => {
  if (!rgb.value.merge_lighting?.enabled) return t("rgb.page.quickSyncOff");
  return t(syncedCount.value === 1 ? "rgb.page.syncOnOne" : "rgb.page.syncOnMany", { count: syncedCount.value });
});

const selectedRaw = ref<string | null>(null);
const selected = computed(() => {
  const wanted = selectedRaw.value;
  if (!rgbCaps.value.length) return null;
  if (wanted === QUICK_SYNC || (wanted && rgbCaps.value.some((cap) => cap.device_id === wanted))) return wanted;
  if (rgb.value.merge_lighting?.enabled) return QUICK_SYNC;
  return rgbCaps.value[0]?.device_id ?? null;
});
const selectedCap = computed(() => rgbCaps.value.find((cap) => cap.device_id === selected.value));

type Tone = "success" | "warning" | "muted" | "info";
function capState(cap: RgbDeviceCapabilities): { label: string; tone: Tone } {
  if (!daemon.connected) return { label: t("common.offline"), tone: "muted" };
  if (cap.deferred_reason) return { label: t("rgb.page.stateWaiting"), tone: "warning" };
  if (openrgbEnabled.value) return { label: "OpenRGB", tone: "info" };
  if (motherboardSyncEnabled(cap, rgb.value)) return { label: t("rgb.page.stateMotherboard"), tone: "info" };
  if (quickSyncControls(cap, rgb.value)) return { label: t("rgb.quickSync"), tone: "info" };
  return { label: t("common.online"), tone: "success" };
}

function isWireless(cap: RgbDeviceCapabilities): boolean {
  return cap.device_id.startsWith("wireless:");
}

function capSummary(cap: RgbDeviceCapabilities): string {
  const count = cap.zones.length;
  if (isWireless(cap)) return t(count === 1 ? "rgb.page.wirelessZoneCountOne" : "rgb.page.wirelessZoneCountMany", { count });
  return t(count === 1 ? "rgb.page.zoneCountOne" : "rgb.page.zoneCountMany", { count });
}
</script>

<template>
  <div class="page rgb-page">
    <AsusAuraControls />
    <details class="card openrgb">
      <summary class="openrgb-title">{{ t("rgb.page.advancedLighting") }} <span class="muted">· OpenRGB</span>
        <span class="status-tag">
          <StatusDot :color="openrgbEnabled ? (daemon.openrgbRunning ? 'success' : 'warning') : 'muted'" />
          {{ statusText }}
        </span>
      </summary>
      <div class="openrgb-row">
        <div class="openrgb-controls">
          <label v-if="openrgbEnabled" class="port-row">
            <span class="muted">{{ t("rgb.page.port") }}</span>
            <n-input-number
              v-model:value="openrgbPort"
              size="small"
              :min="1"
              :max="65535"
            />
          </label>
          <n-checkbox v-model:checked="openrgbEnabled">{{ t("rgb.page.enableOpenrgb") }}</n-checkbox>
        </div>
      </div>
      <template v-if="openrgbEnabled">
        <p class="hint">{{ t("rgb.page.openrgbWarning") }}</p>
        <OpenRgbOptions />
      </template>
    </details>

    <div class="studio-head">
      <div><h2>{{ t("rgb.page.studioTitle") }}</h2><p>{{ t("rgb.page.studioDescription") }}</p></div>
      <div class="sync-overview">
        <span class="sync-pill" :class="{ active: rgb.merge_lighting?.enabled }"><StatusDot :color="rgb.merge_lighting?.enabled ? 'success' : 'muted'" />{{ t("rgb.page.quickSyncState", { state: rgb.merge_lighting?.enabled ? t("common.on") : t("common.off") }) }}</span>
        <span class="sync-pill"><StatusDot :color="motherboardSyncedCount ? 'success' : 'muted'" />{{ t("rgb.page.motherboardSyncCount", { count: motherboardSyncedCount }) }}</span>
      </div>
    </div>
    <div class="layout">
      <aside class="card rgb-list">
        <div class="list-head">
          <span class="list-title">{{ t("rgb.page.lighting") }}</span>
          <span v-if="rgbCaps.length" class="list-count muted">{{ t(rgbCaps.length === 1 ? "rgb.page.deviceCountOne" : "rgb.page.deviceCountMany", { count: rgbCaps.length }) }}</span>
        </div>

        <template v-if="rgbCaps.length">
          <div
            class="rgb-item"
            :class="{ active: selected === QUICK_SYNC }"
            role="button"
            tabindex="0"
            @click="selectedRaw = QUICK_SYNC"
            @keydown.enter="selectedRaw = QUICK_SYNC"
          >
            <Layers :size="16" class="item-icon" />
            <span class="item-text">
              <span class="item-name">{{ t("rgb.quickSync") }}</span>
              <span class="item-sub">{{ syncSummary }}</span>
            </span>
          </div>
          <div class="list-divider" />
        </template>

        <div
          v-for="cap in rgbCaps"
          :key="cap.device_id"
          class="rgb-item"
          :class="{ active: cap.device_id === selected, offline: !daemon.connected }"
          role="button"
          tabindex="0"
          @click="selectedRaw = cap.device_id"
          @keydown.enter="selectedRaw = cap.device_id"
        >
          <component :is="isWireless(cap) ? Wifi : Palette" :size="16" class="item-icon" />
          <span class="item-text">
            <span class="item-name">{{ names.label(cap.device_id, defaultLabel(cap)) }}</span>
            <span class="item-sub">
              <span class="state" :class="`tone-${capState(cap).tone}`" :title="capState(cap).label">
                <span class="dot" />
                <template v-if="capState(cap).tone !== 'success'">{{ capState(cap).label }}</template>
              </span>
              {{ capSummary(cap) }}
            </span>
          </span>
        </div>

        <p v-if="!rgbCaps.length" class="list-empty muted">
          <Palette :size="16" />
          {{ t("rgb.page.noRgbDevices") }}
        </p>
      </aside>

      <div v-if="selected === QUICK_SYNC" class="detail-stack">
        <p class="sync-explainer">{{ t("rgb.page.syncDifference") }}</p>
        <RgbSyncEditor class="detail" />
      </div>
      <RgbDeviceCard
        v-else-if="selectedCap"
        :key="selectedCap.device_id"
        class="detail"
        :cap="selectedCap"
        :label="defaultLabel(selectedCap)"
      />
      <div v-else class="card detail empty muted">
        {{ daemon.connected ? t("rgb.page.noRgbCapableDevices") : t("rgb.page.waitingForDaemon") }}
      </div>
    </div>
  </div>
</template>

<style scoped>
.rgb-page {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}
.openrgb {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  padding: var(--space-3) var(--space-4);
}
.openrgb summary { display:flex; align-items:center; justify-content:space-between; gap:var(--space-3); cursor:pointer; list-style:none; }
.openrgb summary::-webkit-details-marker { display: none; }
.openrgb[open] { border-color: color-mix(in srgb, var(--accent) 28%, var(--border)); }
.studio-head { display:flex; align-items:center; justify-content:space-between; gap:var(--space-4); padding:var(--space-1) 2px; }
.studio-head h2 { margin:0; font-size:var(--font-size-xl); font-weight:650; }
.studio-head p { margin:4px 0 0; color:var(--text-secondary); font-size:var(--font-size-sm); }
.sync-overview { display:flex; flex-wrap:wrap; justify-content:flex-end; gap:var(--space-2); }
.sync-pill { display:inline-flex; align-items:center; gap:7px; padding:7px 10px; border:1px solid var(--border); border-radius:999px; background:var(--bg-surface); color:var(--text-secondary); font-size:var(--font-size-xs); white-space:nowrap; }
.sync-pill.active { color:var(--success); border-color:color-mix(in srgb,var(--success) 35%,var(--border)); }
.detail-stack { min-width:0; display:flex; flex-direction:column; gap:var(--space-2); }
.sync-explainer { margin:0; padding:10px 13px; border-radius:var(--radius-md); border:1px solid color-mix(in srgb,var(--accent) 25%,var(--border)); background:var(--accent-soft); color:var(--text-secondary); font-size:var(--font-size-sm); }
.openrgb-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-2) var(--space-4);
}
.openrgb-title {
  font-weight: 600;
}
.status-tag {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  white-space: nowrap;
}
.openrgb-controls {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  margin-left: auto;
}
.port-row {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  font-size: var(--font-size-sm);
}
.port-row .n-input-number {
  width: 110px;
}
.openrgb .hint {
  margin: 0;
}
.layout {
  display: grid;
  grid-template-columns: minmax(240px, 0.72fr) minmax(0, 2fr);
  gap: var(--space-4);
  align-items: start;
}
.rgb-list {
  position: sticky;
  top: 0;
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  padding: var(--space-3);
}
.list-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: var(--space-2);
}
.list-title {
  font-weight: 600;
}
.list-count {
  font-size: var(--font-size-xs);
}
.list-divider {
  height: 1px;
  margin: var(--space-1) 0;
  background: var(--border);
}
.rgb-item {
  display: flex;
  align-items: flex-start;
  gap: var(--space-2);
  width: 100%;
  padding: var(--space-2);
  border: 1px solid transparent;
  border-radius: var(--radius-md);
  color: var(--text-primary);
  cursor: pointer;
}
.rgb-item:hover {
  background: var(--bg-hover);
}
.rgb-item.active {
  background: var(--accent-soft);
  border-color: color-mix(in srgb, var(--accent) 40%, transparent);
}
.rgb-item.active .item-icon {
  color: var(--accent);
}
.item-icon {
  flex-shrink: 0;
  margin-top: 2px;
  color: var(--text-secondary);
}
.item-text {
  display: flex;
  flex-direction: column;
  min-width: 0;
}
.item-name {
  font-size: var(--font-size-sm);
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.item-sub {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: var(--font-size-xs);
  color: var(--text-secondary);
  overflow: hidden;
  white-space: nowrap;
}
.rgb-item.offline .item-name {
  color: var(--text-muted);
}
.state {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
}
.dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: currentColor;
}
.tone-success { color: var(--success); }
.tone-warning { color: var(--warning); }
.tone-info { color: var(--accent); }
.tone-muted { color: var(--text-muted); }
.list-empty {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin: var(--space-2) 0 0;
  font-size: var(--font-size-sm);
}
.empty {
  padding: var(--space-6);
  text-align: center;
}
@media (max-width: 760px) {
  .studio-head { align-items:flex-start; flex-direction:column; }
  .sync-overview { justify-content:flex-start; }
  .layout {
    grid-template-columns: 1fr;
  }
  .rgb-list {
    position: static;
  }
}
</style>
