<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from "vue";
import { RouterLink } from "vue-router";
import {
  Activity, ArrowDown, ArrowRight, ArrowUp, Cpu, Droplet, Fan, Gauge,
  Image as ImageIcon, Layers3, MemoryStick, Monitor, Palette, Settings2,
} from "lucide-vue-next";
import { useConfigStore } from "@/stores/config";
import { useDaemonStore } from "@/stores/daemon";
import { useDevicesStore } from "@/stores/devices";
import { useIpc } from "@/composables/useIpc";
import { useTranslate } from "@/i18n";
import { liveCoolantTemperature, liveMemoryUsage, liveTemperature, meanFanRpm } from "@/utils/dashboardTelemetry";
import { motherboardSyncEnabled } from "@/components/rgb/syncSettings";
import { finitePercent } from "@/utils/visualization";
import GlassCard from "@/components/dashboard/GlassCard.vue";
import CircularGauge from "@/components/dashboard/CircularGauge.vue";
import LiveSparkline from "@/components/dashboard/LiveSparkline.vue";
import FanTrendChart, { type FanTrendSeries } from "@/components/dashboard/FanTrendChart.vue";

const config = useConfigStore();
const daemon = useDaemonStore();
const devices = useDevicesStore();
const ipc = useIpc();
const t = useTranslate();

const sampleTime = computed(() => daemon.liveSensorSampleTime);
const cpuTemperature = computed(() => daemon.connected
  ? liveTemperature(daemon.liveSensors, "cpu", daemon.liveSensorSampleTime)
  : null);
const gpuTemperature = computed(() => daemon.connected
  ? liveTemperature(daemon.liveSensors, "gpu", daemon.liveSensorSampleTime)
  : null);
const coolantSensor = computed(() => config.config.thermal_alert.coolant_sensor ?? null);
const coolantTemperature = computed(() => {
  if (!daemon.connected) return null;
  if (coolantSensor.value) return liveCoolantTemperature(daemon.liveSensors, coolantSensor.value, daemon.liveSensorSampleTime);
  return devices.aioDevices.map((device) => devices.coolantTemp(device.device_id)).find((value) => value != null) ?? null;
});
function sensorByType(type: string, predicate?: (source: any) => boolean) {
  if (!daemon.connected || daemon.liveSensorSampleTime == null || Date.now() - daemon.liveSensorSampleTime > 10_000) return null;
  return daemon.liveSensors.find((sensor) => sensor.source.type === type && (!predicate || predicate(sensor.source))) ?? null;
}
const cpuLoad = computed(() => finitePercent(sensorByType("cpu_usage")?.current_value));
const gpuLoad = computed(() => finitePercent(sensorByType("nvidia_gpu", (source) => source.metric === "usage")?.current_value));
const memoryUsage = computed(() => daemon.connected
  ? liveMemoryUsage(daemon.liveSensors, daemon.liveSensorSampleTime)
  : null);
const memoryUsed = computed(() => sensorByType("mem_used")?.current_value ?? null);
const memoryFree = computed(() => sensorByType("mem_free")?.current_value ?? null);
const memoryTotal = computed(() => memoryUsed.value != null && memoryFree.value != null
  ? memoryUsed.value + memoryFree.value
  : null);
const cpuSensor = computed(() => daemon.liveSensors.find((sensor) =>
  sensor.source.type === "hwmon" && sensor.source.name === "coretemp" && sensor.source.label === "temp1",
));
const gpuSensor = computed(() => daemon.liveSensors.find((sensor) =>
  sensor.source.type === "nvidia_gpu" && sensor.source.metric === "temp",
));
const coolantSensorInfo = computed(() => coolantSensor.value
  ? daemon.liveSensors.find((sensor) => JSON.stringify(sensor.source) === JSON.stringify(coolantSensor.value)) ?? null
  : null);
const coolantName = computed(() => coolantSensorInfo.value?.sensor_name?.sensor_name ?? "ASUS EC: T_SENSOR");

const networkRx = computed(() => {
  if (!daemon.connected) return null;
  const values = daemon.liveSensors.filter((sensor) => sensor.source.type === "network_rate" && sensor.source.direction === "rx");
  return values.length ? values.reduce((sum, sensor) => sum + (sensor.current_value != null && Number.isFinite(sensor.current_value) ? sensor.current_value : 0), 0) : null;
});
const networkTx = computed(() => {
  if (!daemon.connected) return null;
  const values = daemon.liveSensors.filter((sensor) => sensor.source.type === "network_rate" && sensor.source.direction === "tx");
  return values.length ? values.reduce((sum, sensor) => sum + (sensor.current_value != null && Number.isFinite(sensor.current_value) ? sensor.current_value : 0), 0) : null;
});

const averageFanRpm = computed(() => daemon.connected
  ? meanFanRpm(devices.fanDevices.flatMap((device) => devices.fanRpms(device.device_id)))
  : null);
const fanFilter = ref("all");
const fanColors = ["#ff765a", "#18c6ff", "#e44dff", "#ffbd4a", "#63e2a8", "#9b86ff"];
function shortDeviceName(name: string, index: number): string {
  const port = name.match(/Port\s+(\d+)/i)?.[1];
  return port != null ? `Port ${port}` : name.length > 18 ? `${name.slice(0, 16)}…` : name || `Fan ${index + 1}`;
}
const allFanSeries = computed<FanTrendSeries[]>(() => devices.fanDevices.map((device, index) => ({
  id: device.device_id,
  label: shortDeviceName(device.name, index),
  color: fanColors[index % fanColors.length],
  rpm: daemon.connected ? meanFanRpm(devices.fanRpms(device.device_id)) : null,
})));
const fanSeries = computed(() => fanFilter.value === "all"
  ? allFanSeries.value
  : allFanSeries.value.filter((series) => series.id === fanFilter.value));
const fanFilterLabel = computed(() => fanFilter.value === "all"
  ? t("dashboard.allFans")
  : allFanSeries.value.find((series) => series.id === fanFilter.value)?.label ?? t("dashboard.fanSpeeds"));

const rgbDevices = computed(() => config.rgbCaps.length);
const syncDevices = computed(() => config.rgbCaps.filter((capability) =>
  config.config.rgb ? motherboardSyncEnabled(capability, config.config.rgb) : false,
).length);
const activeEffect = computed(() => {
  if (config.config.asus_aura.rainbow_enabled && daemon.auraLighting?.worker_state === "streaming") return t("dashboard.rainbow");
  if (config.config.rgb?.merge_lighting?.enabled) return t("dashboard.motherboardSync");
  const effect = config.config.rgb?.devices.flatMap((device) => device.zones).find((zone) => !zone.effect.disabled)?.effect;
  return effect?.mode ?? t("dashboard.noEffect");
});
const rgbColors = computed(() => (config.config.rgb?.devices ?? [])
  .flatMap((device) => device.zones ?? [])
  .flatMap((zone) => zone.effect.colors ?? [])
  .slice(0, 5)
  .map((color) => `rgb(${color.join(",")})`));
const hasRainbowEffect = computed(() => /rainbow/i.test(String(activeEffect.value)));

const lcdDevice = computed(() => devices.lcdDevices[0] ?? null);
const lcdConfig = computed(() => {
  const entries = config.config.lcds;
  if (!entries.length) return null;
  const bySerial = entries.find((entry) => entry.serial && (entry.serial === lcdDevice.value?.device_id || entry.serial === lcdDevice.value?.serial));
  return bySerial ?? entries[0];
});
const lcdFilename = computed(() => {
  const path = lcdConfig.value?.path;
  const segments = path?.split(/[\\/]/).filter(Boolean) ?? [];
  return segments[segments.length - 1] ?? null;
});
const lcdDisplayLabel = computed(() => {
  const filename = lcdFilename.value;
  return !filename || /^[a-f0-9]{40,}(?:\.[a-z0-9]+)?$/i.test(filename)
    ? t("dashboard.currentLcdMedia")
    : filename;
});
const lcdStatus = computed(() => {
  if (!daemon.connected) return t("dashboard.offline");
  if (!lcdDevice.value) return t("dashboard.notDetected");
  const index = lcdConfig.value ? config.config.lcds.indexOf(lcdConfig.value) : -1;
  const preparation = index >= 0 ? daemon.mediaPreparation[String(index)] : null;
  if (preparation?.state === "failed") return t("dashboard.needsAttention");
  if (preparation?.state === "preparing") return t("dashboard.preparingMedia");
  if (preparation?.state === "waiting_for_device") return t("dashboard.waitingForDisplay");
  if (preparation?.state === "ready") return t("dashboard.mediaPrepared");
  return lcdFilename.value ? t("dashboard.detected") : t("dashboard.noMediaConfigured");
});
const lcdBrightness = computed(() => lcdDevice.value
  ? devices.telemetry.lcd_brightness?.[lcdDevice.value.device_id]?.brightness ?? null
  : null);
const lcdPreviewUrl = ref("");
const lcdPreviewUnavailable = ref(false);
let previewRevision = 0;
watch(() => [
  lcdConfig.value?.path ?? "",
  lcdConfig.value?.type ?? "",
  lcdConfig.value ? config.config.lcds.indexOf(lcdConfig.value) : -1,
  daemon.connected,
] as const, async ([path, type, index, connected]) => {
  const revision = ++previewRevision;
  if (lcdPreviewUrl.value) URL.revokeObjectURL(lcdPreviewUrl.value);
  lcdPreviewUrl.value = "";
  lcdPreviewUnavailable.value = false;
  if (!path || type !== "image" || !connected || index < 0) return;
  try {
    const { jpeg_base64: encoded } = await ipc.request<{ jpeg_base64: string }>("RenderLcdMediaPreview", {
      index,
      width: 640,
      height: 360,
    });
    if (revision !== previewRevision) return;
    const binary = atob(encoded);
    const bytes = Uint8Array.from(binary, (character) => character.charCodeAt(0));
    lcdPreviewUrl.value = URL.createObjectURL(new Blob([bytes], { type: "image/jpeg" }));
  } catch {
    if (revision === previewRevision) lcdPreviewUnavailable.value = true;
  }
}, { immediate: true });
onUnmounted(() => {
  previewRevision++;
  if (lcdPreviewUrl.value) URL.revokeObjectURL(lcdPreviewUrl.value);
});

const devicesForDisplay = computed(() => devices.visible.slice(0, 5));
function deviceKind(device: (typeof devices.visible)[number]): string {
  if (device.has_lcd) return "LCD";
  if (device.has_fan) return t("dashboard.fanController");
  if (device.has_rgb) return t("dashboard.lighting");
  return device.family;
}
function deviceRpm(deviceId: string): number | null {
  return meanFanRpm(devices.fanRpms(deviceId));
}
function formatRpm(value: number | null): string {
  return value == null ? "—" : `${Math.round(value).toLocaleString()} RPM`;
}
function formatPercent(value: number | null): string {
  return value == null ? "—" : `${Math.round(value)}%`;
}
function metricBar(value: number | null): string {
  return value == null ? "0%" : `${Math.max(0, Math.min(100, value))}%`;
}
</script>

<template>
  <main class="dashboard-page">
    <section class="welcome-row">
      <div class="welcome-copy">
        <h2>{{ t("dashboard.welcome") }}</h2>
        <p>{{ t("dashboard.welcomeSubtitle") }}</p>
      </div>
      <div class="welcome-tools">
        <div class="rig-pill">
          <span class="rig-icon"><Monitor :size="18" /></span>
          <span class="rig-name">{{ t("dashboard.thisSystem") }}</span>
          <span class="rig-divider" />
          <span class="daemon-state" :class="daemon.connected ? 'is-connected' : 'is-offline'">
            <i />{{ daemon.connected ? t("dashboard.daemonOnline") : t("dashboard.daemonOffline") }}
          </span>
        </div>
        <RouterLink to="/settings" class="icon-link" :aria-label="t('nav.settings')" :title="t('nav.settings')"><Settings2 :size="17" /></RouterLink>
      </div>
    </section>

    <section class="metric-grid" :aria-label="t('dashboard.liveMetrics')">
      <GlassCard tone="pink" class="metric-card">
        <header class="metric-heading"><span class="metric-icon"><Cpu :size="19" /></span><span>{{ t("dashboard.cpu") }}</span><span class="more-mark">•••</span></header>
        <div class="metric-body">
          <CircularGauge :value="cpuTemperature" :max="100" tone="pink" label="CPU" unit="°C" />
          <div class="metric-detail">
            <div class="metric-chart"><LiveSparkline :value="cpuTemperature" :sample-time="sampleTime" tone="pink" /></div>
            <div class="detail-label">{{ t("dashboard.cpuLoad") }}</div><strong class="detail-value">{{ formatPercent(cpuLoad) }}</strong>
            <div class="progress-track"><i class="tone-pink-fill" :style="{ width: metricBar(cpuLoad) }" /></div>
            <small class="sensor-caption">{{ cpuTemperature == null ? t("dashboard.sensorUnavailable") : cpuSensor?.sensor_name?.sensor_name ?? t("dashboard.liveTemperature") }}</small>
          </div>
        </div>
      </GlassCard>

      <GlassCard tone="cyan" class="metric-card">
        <header class="metric-heading"><span class="metric-icon"><Layers3 :size="19" /></span><span>{{ t("dashboard.gpu") }}</span><span class="more-mark">•••</span></header>
        <div class="metric-body">
          <CircularGauge :value="gpuTemperature" :max="100" tone="cyan" label="GPU" unit="°C" />
          <div class="metric-detail">
            <div class="metric-chart"><LiveSparkline :value="gpuTemperature" :sample-time="sampleTime" tone="cyan" /></div>
            <div class="detail-label">{{ t("dashboard.gpuLoad") }}</div><strong class="detail-value">{{ formatPercent(gpuLoad) }}</strong>
            <div class="progress-track"><i class="tone-cyan-fill" :style="{ width: metricBar(gpuLoad) }" /></div>
            <small class="sensor-caption">{{ gpuSensor?.sensor_name?.device_name ?? (gpuTemperature == null ? t("dashboard.sensorUnavailable") : t("dashboard.liveTemperature")) }}</small>
          </div>
        </div>
      </GlassCard>

      <GlassCard tone="blue" class="metric-card">
        <header class="metric-heading"><span class="metric-icon"><Droplet :size="19" /></span><span>{{ t("dashboard.coolant") }}</span><span class="more-mark">•••</span></header>
        <div class="metric-body">
          <CircularGauge :value="coolantTemperature" :max="100" tone="blue" label="WATER" unit="°C" />
          <div class="metric-detail">
            <div class="metric-chart"><LiveSparkline :value="coolantTemperature" :sample-time="sampleTime" tone="blue" /></div>
            <div class="detail-label">{{ t("dashboard.coolantSource") }}</div><strong class="detail-value">{{ coolantTemperature == null ? "—" : `${coolantTemperature.toFixed(0)}°C` }}</strong>
            <div class="source-pill"><span class="source-dot" />{{ coolantSensorInfo ? "ASUS EC · T_SENSOR" : t("dashboard.sensorUnavailable") }}</div>
            <small class="sensor-caption">{{ coolantName }}</small>
          </div>
        </div>
      </GlassCard>

      <GlassCard tone="violet" class="metric-card">
        <header class="metric-heading"><span class="metric-icon"><MemoryStick :size="19" /></span><span>{{ t("dashboard.memory") }}</span><span class="more-mark">•••</span></header>
        <div class="metric-body">
          <CircularGauge :value="memoryUsage" :max="100" tone="violet" label="RAM" unit="%" />
          <div class="metric-detail">
            <div class="metric-chart"><LiveSparkline :value="memoryUsage" :sample-time="sampleTime" tone="violet" /></div>
            <div class="detail-label">{{ t("dashboard.memoryUsed") }}</div><strong class="detail-value">{{ memoryUsed == null || memoryTotal == null ? "—" : `${memoryUsed.toFixed(1)} / ${memoryTotal.toFixed(1)} GB` }}</strong>
            <div class="progress-track"><i class="tone-violet-fill" :style="{ width: metricBar(memoryUsage) }" /></div>
            <small class="sensor-caption">{{ memoryUsage == null ? t("dashboard.sensorUnavailable") : t("dashboard.liveUsage") }}</small>
          </div>
        </div>
      </GlassCard>
    </section>

    <section class="middle-grid">
      <GlassCard tone="pink" class="panel fan-panel">
        <header class="panel-heading">
          <div class="panel-title"><span class="panel-icon fan-tone"><Fan :size="18" /></span><div><span class="panel-kicker">{{ t("dashboard.cooling") }}</span><h3>{{ t("dashboard.fanSpeeds") }}</h3></div></div>
          <RouterLink class="panel-action" to="/fans">{{ t("dashboard.openFanControls") }} <ArrowRight :size="14" /></RouterLink>
        </header>
        <div class="fan-filter" role="group" :aria-label="t('dashboard.filterFans')">
          <button type="button" :aria-pressed="fanFilter === 'all'" :class="{ selected: fanFilter === 'all' }" @click="fanFilter = 'all'">{{ t("dashboard.allFans") }}</button>
          <button v-for="series in allFanSeries" :key="series.id" type="button" :title="series.label" :aria-pressed="fanFilter === series.id" :class="{ selected: fanFilter === series.id }" @click="fanFilter = series.id">{{ series.label }}</button>
        </div>
        <FanTrendChart :series="fanSeries" :sample-time="sampleTime" />
        <div v-if="fanSeries.length" class="fan-legend">
          <div v-for="series in fanSeries" :key="series.id" class="legend-item"><i :style="{ background: series.color, boxShadow: `0 0 10px ${series.color}` }" /><span :title="series.label">{{ series.label }}</span><strong>{{ formatRpm(series.rpm) }}</strong></div>
        </div>
        <div v-else class="panel-empty"><Fan :size="18" />{{ daemon.connected ? t("dashboard.noFansDetected") : t("dashboard.connectDaemon") }}</div>
      </GlassCard>

      <GlassCard tone="violet" class="panel rgb-panel">
        <header class="panel-heading">
          <div class="panel-title"><span class="panel-icon rgb-tone"><Palette :size="18" /></span><div><span class="panel-kicker">{{ t("dashboard.lighting") }}</span><h3>{{ t("dashboard.rgbControl") }}</h3></div></div>
          <RouterLink class="panel-arrow" to="/rgb" :aria-label="t('dashboard.openRgbControls')"><ArrowRight :size="18" /></RouterLink>
        </header>
        <div class="rgb-display">
          <div class="rgb-orbit" :class="{ spectrum: hasRainbowEffect }" :style="{ '--saved-rgb': rgbColors[0] ?? 'var(--neon-violet)' }"><span><Palette :size="25" /></span></div>
          <div class="rgb-summary"><span class="panel-kicker">{{ t("dashboard.savedEffect") }}</span><strong>{{ activeEffect }}</strong><small>{{ rgbDevices }} {{ t("dashboard.controllersShort") }} · {{ syncDevices }} {{ t("dashboard.devicesSynced") }}</small></div>
        </div>
        <div v-if="hasRainbowEffect" class="rgb-swatches spectrum-swatches" :aria-label="t('dashboard.savedEffect')"><span v-for="color in ['#ff3fb4','#ffac27','#38e99b','#16d7eb','#368eff']" :key="color" :style="{ background: color }" /></div>
        <div v-else-if="rgbColors.length" class="rgb-swatches" :aria-label="t('dashboard.savedColors')"><span v-for="(color,index) in rgbColors" :key="`${color}-${index}`" :style="{ background: color }" /></div>
        <div v-else class="rgb-swatches empty-swatches"><span v-for="n in 5" :key="n" /></div>
        <RouterLink class="wide-action" to="/rgb"><span><Palette :size="15" />{{ t("dashboard.openRgbControls") }}</span><ArrowRight :size="15" /></RouterLink>
      </GlassCard>

      <GlassCard tone="cyan" class="panel lcd-panel">
        <header class="panel-heading">
          <div class="panel-title"><span class="panel-icon lcd-tone"><Monitor :size="18" /></span><div><span class="panel-kicker">{{ t("dashboard.display") }}</span><h3>{{ t("dashboard.lcd") }}</h3></div></div>
          <RouterLink class="panel-arrow" to="/lcd" :aria-label="t('dashboard.openLcdControls')"><ArrowRight :size="18" /></RouterLink>
        </header>
        <div class="lcd-preview-frame">
          <img v-if="lcdPreviewUrl" :src="lcdPreviewUrl" :alt="lcdFilename ?? t('dashboard.currentLcdMedia')" />
          <div v-else class="lcd-placeholder"><ImageIcon :size="29" /><span>{{ lcdPreviewUnavailable ? t("dashboard.previewUnavailable") : lcdDisplayLabel }}</span></div>
          <div class="lcd-overlay"><span class="live-badge"><i />{{ lcdStatus }}</span></div>
        </div>
        <div class="lcd-meta">
          <div class="lcd-device"><strong>{{ lcdDevice?.name ?? t("dashboard.noLcdDetected") }}</strong><small>{{ lcdConfig?.type ? lcdConfig.type.toUpperCase() : t("dashboard.noMediaConfigured") }}<template v-if="lcdBrightness != null"> · {{ t("dashboard.brightness") }} {{ lcdBrightness }}%</template></small></div>
          <RouterLink class="lcd-open" to="/lcd">{{ t("dashboard.openLcdControls") }}<ArrowRight :size="14" /></RouterLink>
        </div>
      </GlassCard>
    </section>

    <section class="bottom-grid">
      <GlassCard tone="pink" class="panel device-panel">
        <header class="panel-heading">
          <div class="panel-title"><span class="panel-icon devices-tone"><Layers3 :size="18" /></span><div><span class="panel-kicker">{{ t("dashboard.hardware") }}</span><h3>{{ t("dashboard.connectedDevices") }}</h3></div></div>
          <span class="detected-count">{{ devices.visible.length }} {{ t("dashboard.detected") }}</span>
        </header>
        <div v-if="!daemon.connected" class="panel-empty"><Activity :size="18" />{{ t("dashboard.connectDaemon") }}</div>
        <div v-else-if="!devicesForDisplay.length" class="panel-empty"><Activity :size="18" />{{ t("dashboard.noControllers") }}</div>
        <div v-else class="device-list">
          <div v-for="device in devicesForDisplay" :key="device.device_id" class="device-row">
            <span class="device-glyph" :class="device.has_lcd ? 'cyan' : device.has_fan ? 'pink' : 'violet'"><ImageIcon v-if="device.has_lcd" :size="15" /><Fan v-else-if="device.has_fan" :size="15" /><Palette v-else :size="15" /></span>
            <span class="device-copy"><strong>{{ device.name }}</strong><small>{{ deviceKind(device) }}</small></span>
            <span class="device-value" v-if="device.has_fan">{{ formatRpm(deviceRpm(device.device_id)) }}</span>
            <span class="device-online"><i />{{ t("dashboard.connected") }}</span>
          </div>
        </div>
        <RouterLink class="panel-footer-link" to="/devices">{{ t("dashboard.allHardware") }} <ArrowRight :size="14" /></RouterLink>
      </GlassCard>

      <GlassCard tone="cyan" class="panel performance-panel">
        <header class="panel-heading performance-heading">
          <div class="panel-title"><span class="performance-icon"><Gauge :size="20" /></span><div><span class="panel-kicker">{{ t("dashboard.liveTelemetry") }}</span><h3>{{ t("dashboard.systemPerformance") }}</h3></div></div>
          <span class="sample-status" :class="{ active: sampleTime != null && Date.now() - sampleTime < 10000 }"><i />{{ sampleTime != null && Date.now() - sampleTime < 10000 ? t("dashboard.live") : t("dashboard.waiting") }}</span>
        </header>
        <div class="performance-grid">
          <div class="performance-tile"><span class="tile-label">{{ t("dashboard.cpuLoad") }}</span><strong class="pink-text">{{ formatPercent(cpuLoad) }}</strong><LiveSparkline :value="cpuLoad" :sample-time="sampleTime" tone="pink" /></div>
          <div class="performance-tile"><span class="tile-label">{{ t("dashboard.gpuLoad") }}</span><strong class="cyan-text">{{ formatPercent(gpuLoad) }}</strong><LiveSparkline :value="gpuLoad" :sample-time="sampleTime" tone="cyan" /></div>
          <div class="performance-tile"><span class="tile-label">{{ t("dashboard.memory") }}</span><strong class="violet-text">{{ formatPercent(memoryUsage) }}</strong><LiveSparkline :value="memoryUsage" :sample-time="sampleTime" tone="violet" /></div>
          <div class="performance-tile network-tile"><span class="tile-label">{{ t("dashboard.network") }}</span><strong><span class="cyan-text"><ArrowUp :size="13" /> {{ networkRx == null ? "—" : `${networkRx.toFixed(2)} MB/s` }}</span><span class="pink-text"><ArrowDown :size="13" /> {{ networkTx == null ? "—" : `${networkTx.toFixed(2)} MB/s` }}</span></strong><LiveSparkline :value="networkRx" :sample-time="sampleTime" tone="cyan" /></div>
        </div>
        <div class="performance-foot"><span><i class="cpu-dot" />{{ cpuTemperature == null ? "—" : `${cpuTemperature.toFixed(0)}°C` }} {{ t("dashboard.cpu") }}</span><span><i class="gpu-dot" />{{ gpuTemperature == null ? "—" : `${gpuTemperature.toFixed(0)}°C` }} {{ t("dashboard.gpu") }}</span><span><i class="coolant-dot" />{{ coolantTemperature == null ? "—" : `${coolantTemperature.toFixed(0)}°C` }} {{ t("dashboard.coolant") }}</span></div>
      </GlassCard>
    </section>
  </main>
</template>

<style scoped>
.dashboard-page { width:100%; max-width:1500px; min-height:100%; margin:0 auto; display:flex; flex-direction:column; gap:15px; }
.welcome-row { display:flex; align-items:center; justify-content:space-between; gap:18px; min-height:73px; padding:0 2px 2px; }
.welcome-copy { min-width:0; }
.eyebrow { display:flex; align-items:center; gap:8px; color:#7eb7ff; font-size:10px; font-weight:750; letter-spacing:.14em; text-transform:uppercase; }
.welcome-copy h2 { margin:0 0 5px; font-size:clamp(25px,2.4vw,32px); line-height:1.05; letter-spacing:-.045em; font-weight:690; }
.welcome-copy p { margin:0; color:var(--text-secondary); font-size:12px; }
.welcome-tools { display:flex; align-items:center; gap:10px; }
.rig-pill { min-height:42px; display:flex; align-items:center; gap:11px; padding:0 14px; border:1px solid rgba(112,166,245,.30); border-radius:14px; background:linear-gradient(135deg,rgba(33,60,96,.46),rgba(10,23,42,.78)); box-shadow:inset 0 1px rgba(255,255,255,.07),0 8px 24px rgba(0,0,0,.17); }
.rig-icon { display:grid; place-items:center; color:#c3ddff; }
.rig-name { font-size:12px; font-weight:620; white-space:nowrap; }
.rig-divider { width:1px; height:22px; background:rgba(160,190,233,.19); }
.daemon-state { display:flex; align-items:center; gap:7px; font-size:11px; color:#e5efff; white-space:nowrap; }
.daemon-state i,.live-badge i,.device-online i,.sample-status i { width:7px; height:7px; border-radius:50%; background:var(--success); box-shadow:0 0 11px color-mix(in srgb,var(--success) 75%,transparent); }
.daemon-state.is-offline i { background:var(--danger); box-shadow:0 0 10px color-mix(in srgb,var(--danger) 60%,transparent); }
.icon-link,.panel-arrow { width:38px; height:38px; display:grid; place-items:center; border:1px solid rgba(140,177,227,.24); border-radius:12px; color:var(--text-secondary); background:rgba(13,28,48,.56); transition:color .16s,border-color .16s,box-shadow .16s; }
.icon-link:hover,.panel-arrow:hover { color:#fff; border-color:rgba(93,177,255,.65); box-shadow:0 0 18px rgba(43,150,255,.2); }
.metric-grid { display:grid; grid-template-columns:repeat(4,minmax(0,1fr)); gap:13px; }
.metric-card { min-height:222px; padding:14px 15px 12px; }
.metric-heading { display:flex; align-items:center; gap:9px; color:#e9f2ff; font-size:12px; font-weight:620; }
.metric-icon { display:grid; place-items:center; color:var(--neon-blue); }
.tone-pink .metric-icon { color:var(--neon-pink); }.tone-cyan .metric-icon { color:var(--neon-cyan); }.tone-violet .metric-icon { color:var(--neon-violet); }
.more-mark { margin-left:auto; color:rgba(203,221,249,.55); letter-spacing:2px; font-size:11px; }
.metric-body { display:flex; align-items:center; justify-content:space-between; gap:5px; min-height:159px; }
.metric-detail { min-width:0; flex:1; display:flex; flex-direction:column; align-items:flex-start; padding:3px 0 0 3px; }
.metric-chart { width:100%; height:40px; margin-bottom:7px; }
.detail-label { color:var(--text-muted); font-size:9px; letter-spacing:.035em; text-transform:uppercase; }
.detail-value { margin-top:2px; color:#f3f7ff; font-size:14px; font-weight:650; line-height:1.25; font-variant-numeric:tabular-nums; white-space:nowrap; }
.progress-track { width:100%; height:6px; margin-top:6px; overflow:hidden; border-radius:99px; background:rgba(105,138,180,.16); box-shadow:inset 0 1px 2px rgba(0,0,0,.34); }
.progress-track i { display:block; height:100%; min-width:2px; border-radius:inherit; transition:width .35s ease; }
.tone-pink-fill { background:linear-gradient(90deg,#9b4dff,#ff4cb7); box-shadow:0 0 12px #ed55e4; }.tone-cyan-fill { background:linear-gradient(90deg,#168be9,#19e4e6); box-shadow:0 0 12px #19c8ff; }.tone-violet-fill { background:linear-gradient(90deg,#634dff,#ce65ff); box-shadow:0 0 12px #a55dff; }
.sensor-caption { width:100%; margin-top:7px; overflow:hidden; color:var(--text-muted); font-size:8px; text-overflow:ellipsis; white-space:nowrap; }
.source-pill { display:flex; align-items:center; gap:5px; margin-top:5px; color:#9fcaff; font-size:8px; white-space:nowrap; }
.source-dot { width:5px; height:5px; border-radius:50%; background:var(--neon-cyan); box-shadow:0 0 7px var(--neon-cyan); }
.middle-grid { display:grid; grid-template-columns:minmax(0,1.48fr) minmax(210px,.9fr) minmax(250px,1.12fr); gap:13px; }
.panel { min-height:322px; padding:14px 16px 13px; }
.panel-heading { min-height:37px; display:flex; align-items:center; justify-content:space-between; gap:10px; margin-bottom:10px; }
.panel-title { min-width:0; display:flex; align-items:center; gap:10px; }
.panel-icon { width:31px; height:31px; flex:0 0 auto; display:grid; place-items:center; border-radius:10px; }
.fan-tone { color:#ff72b5; background:rgba(255,90,164,.12); }.rgb-tone { color:#e174ff; background:rgba(190,87,255,.14); }.lcd-tone { color:#38d9ff; background:rgba(42,197,255,.13); }.devices-tone { color:#d276ff; background:rgba(194,95,255,.13); }
.panel-kicker { display:block; margin-bottom:3px; color:var(--text-muted); font-size:8px; font-weight:700; letter-spacing:.13em; text-transform:uppercase; }
.panel-heading h3 { margin:0; color:#eaf2ff; font-size:14px; font-weight:620; letter-spacing:-.015em; line-height:1.15; }
.panel-action { display:flex; align-items:center; gap:5px; color:#8dbbff; font-size:9px; white-space:nowrap; }
.panel-action:hover,.lcd-open:hover,.panel-footer-link:hover { color:white; }
.fan-filter { display:flex; gap:3px; width:max-content; max-width:100%; overflow-x:auto; padding:3px; border:1px solid rgba(131,167,216,.14); border-radius:10px; background:rgba(5,14,27,.42); }
.fan-filter button { min-height:25px; padding:0 9px; border:0; border-radius:7px; color:var(--text-muted); background:transparent; cursor:pointer; font:inherit; font-size:9px; white-space:nowrap; }
.fan-filter button:hover { color:var(--text-primary); }.fan-filter button.selected { color:#eef6ff; background:linear-gradient(135deg,rgba(42,119,255,.72),rgba(101,69,255,.62)); box-shadow:0 2px 10px rgba(35,105,255,.28),inset 0 1px rgba(255,255,255,.22); }
.fan-legend { display:grid; grid-template-columns:repeat(2,minmax(0,1fr)); gap:6px 11px; margin-top:5px; }
.legend-item { min-width:0; display:flex; align-items:center; gap:6px; color:var(--text-secondary); font-size:8px; }
.legend-item i { width:6px; height:6px; flex:0 0 auto; border-radius:50%; }
.legend-item span { min-width:0; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
.legend-item strong { margin-left:auto; color:#dce9fa; font:9px var(--font-mono); white-space:nowrap; }
.panel-empty { min-height:140px; display:flex; align-items:center; justify-content:center; gap:8px; color:var(--text-muted); font-size:11px; }
.panel-arrow { width:29px; height:29px; border-radius:9px; }
.rgb-panel { display:flex; flex-direction:column; }
.rgb-display { display:flex; align-items:center; gap:13px; min-height:156px; }
.rgb-orbit { width:140px; height:140px; flex:0 0 auto; position:relative; display:grid; place-items:center; border-radius:50%; background:conic-gradient(from -42deg,#22e0ef,#2e79ff,#9b4cff,#ff56ae,#ffb636,#22d984,#22e0ef); filter:drop-shadow(0 0 12px rgba(68,110,255,.35)); }
.rgb-orbit::before { content:""; position:absolute; inset:11px; border:1px solid rgba(255,255,255,.28); border-radius:50%; background:rgba(7,17,31,.89); box-shadow:inset 0 0 28px rgba(62,112,190,.34),0 0 13px rgba(89,135,255,.30); }
.rgb-orbit::after { content:""; position:absolute; inset:26px; border:1px solid rgba(159,195,241,.20); border-radius:50%; }
.rgb-orbit span { position:relative; z-index:1; display:grid; place-items:center; color:#d8eaff; }
.rgb-orbit:not(.spectrum) { background:conic-gradient(from -45deg,var(--saved-rgb),color-mix(in srgb,var(--saved-rgb) 60%,transparent) 78%,transparent 78%); }
.rgb-summary { min-width:0; display:flex; flex-direction:column; gap:5px; }
.rgb-summary strong { color:#f2f5ff; font-size:13px; font-weight:650; overflow-wrap:anywhere; }
.rgb-summary small { color:var(--text-muted); font-size:9px; line-height:1.35; }
.rgb-swatches { display:flex; gap:7px; min-height:17px; margin:-1px 0 10px 6px; }
.rgb-swatches span { width:15px; height:15px; border:1px solid rgba(255,255,255,.38); border-radius:50%; box-shadow:0 0 9px color-mix(in srgb,currentColor 30%,transparent); }
.empty-swatches span { background:linear-gradient(140deg,rgba(116,143,180,.16),rgba(68,91,124,.26)); border-color:rgba(161,187,221,.13); }
.spectrum-swatches span { box-shadow:0 0 12px color-mix(in srgb,currentColor 50%,transparent); }
.wide-action { min-height:32px; display:flex; align-items:center; justify-content:space-between; gap:8px; margin-top:auto; padding:0 10px; border:1px solid rgba(161,125,255,.28); border-radius:9px; color:#e2d5ff; background:linear-gradient(100deg,rgba(110,74,191,.20),rgba(52,81,151,.19)); font-size:9px; }
.wide-action span { display:flex; align-items:center; gap:7px; }.wide-action:hover { color:white; border-color:rgba(187,140,255,.58); }
.lcd-preview-frame { height:157px; position:relative; display:flex; align-items:center; justify-content:center; overflow:hidden; border:1px solid rgba(93,185,255,.44); border-radius:12px; background:linear-gradient(135deg,rgba(30,65,99,.5),rgba(6,16,30,.9)); box-shadow:inset 0 0 22px rgba(20,160,255,.13),0 0 18px rgba(20,148,255,.10); }
.lcd-preview-frame::before { content:""; position:absolute; inset:5px; border:1px solid rgba(178,223,255,.12); border-radius:8px; pointer-events:none; }
.lcd-preview-frame img { width:100%; height:100%; object-fit:contain; }
.lcd-placeholder { max-width:85%; display:flex; flex-direction:column; align-items:center; gap:8px; color:#7fa4c8; text-align:center; }
.lcd-placeholder span { overflow:hidden; max-width:100%; color:#c8d8eb; font-size:9px; text-overflow:ellipsis; white-space:nowrap; }
.lcd-overlay { position:absolute; left:10px; bottom:9px; }
.live-badge { display:flex; align-items:center; gap:5px; min-height:19px; padding:0 7px; border:1px solid rgba(110,202,161,.27); border-radius:999px; background:rgba(4,21,26,.70); color:#b8f7dc; font-size:8px; }
.live-badge i { width:5px; height:5px; }
.lcd-meta { min-width:0; display:flex; align-items:center; justify-content:space-between; gap:9px; padding-top:10px; }
.lcd-device { min-width:0; display:flex; flex-direction:column; gap:3px; }
.lcd-device strong { max-width:175px; overflow:hidden; color:#e7f2ff; font-size:9px; text-overflow:ellipsis; white-space:nowrap; }
.lcd-device small { color:var(--text-muted); font-size:8px; }
.lcd-open { display:flex; align-items:center; gap:4px; color:#83dfff; font-size:8px; white-space:nowrap; }
.bottom-grid { display:grid; grid-template-columns:minmax(280px,.91fr) minmax(0,1.48fr); gap:13px; }
.device-panel,.performance-panel { min-height:274px; }
.detected-count { color:var(--text-muted); font-size:9px; white-space:nowrap; }
.device-list { display:flex; flex-direction:column; }
.device-row { min-width:0; min-height:30px; display:flex; align-items:center; gap:8px; border-top:1px solid rgba(149,179,219,.10); }
.device-glyph { width:24px; height:24px; flex:0 0 auto; display:grid; place-items:center; border-radius:7px; background:rgba(85,125,185,.14); }
.device-glyph.pink { color:#ff86be; background:rgba(255,93,174,.10); }.device-glyph.cyan { color:#4bdcff; background:rgba(39,198,255,.10); }.device-glyph.violet { color:#ce8cff; background:rgba(178,95,255,.11); }
.device-copy { min-width:0; flex:1; display:flex; align-items:center; gap:8px; }
.device-copy strong { min-width:0; max-width:65%; overflow:hidden; color:#e6eefb; font-size:9px; font-weight:560; text-overflow:ellipsis; white-space:nowrap; }
.device-copy small { overflow:hidden; color:var(--text-muted); font-size:8px; text-overflow:ellipsis; white-space:nowrap; }
.device-value { color:#d8e6f7; font:8px var(--font-mono); white-space:nowrap; }
.device-online { display:flex; align-items:center; gap:4px; color:#adbdcf; font-size:8px; white-space:nowrap; }
.device-online i { width:5px; height:5px; }
.panel-footer-link { display:flex; align-items:center; justify-content:space-between; margin-top:5px; padding-top:8px; border-top:1px solid rgba(149,179,219,.12); color:#8cb9ef; font-size:9px; }
.performance-panel { display:flex; flex-direction:column; }
.performance-heading { margin-bottom:8px; }
.performance-icon { color:#46cfff; filter:drop-shadow(0 0 7px rgba(31,198,255,.6)); }
.sample-status { display:flex; align-items:center; gap:5px; color:var(--text-muted); font-size:8px; text-transform:uppercase; letter-spacing:.08em; }
.sample-status i { width:5px; height:5px; background:#61728b; box-shadow:none; }
.sample-status.active i { background:#33dfa6; box-shadow:0 0 8px #33dfa6; }
.performance-grid { display:grid; grid-template-columns:repeat(4,minmax(0,1fr)); gap:8px; }
.performance-tile { min-width:0; min-height:99px; display:flex; flex-direction:column; padding:8px 9px 6px; border:1px solid rgba(88,147,207,.20); border-radius:10px; background:linear-gradient(145deg,rgba(17,39,67,.68),rgba(7,17,31,.72)); box-shadow:inset 0 1px rgba(255,255,255,.04); }
.tile-label { color:#a8bad2; font-size:8px; white-space:nowrap; }
.performance-tile strong { min-height:32px; display:flex; align-items:center; gap:5px; color:#f3f7ff; font-size:18px; font-weight:670; font-variant-numeric:tabular-nums; letter-spacing:-.035em; white-space:nowrap; }
.performance-tile .sparkline { height:29px; margin-top:auto; }
.pink-text { color:#ff69c6!important; }.cyan-text { color:#37d6ff!important; }.violet-text { color:#c07bff!important; }
.network-tile strong { align-items:flex-start; flex-direction:column; justify-content:center; gap:0; font-size:9px; letter-spacing:0; }
.network-tile strong span { display:flex; align-items:center; gap:3px; }
.performance-foot { display:flex; align-items:center; flex-wrap:wrap; gap:13px; margin-top:auto; padding-top:8px; color:var(--text-muted); font-size:8px; }
.performance-foot span { display:flex; align-items:center; gap:5px; }
.performance-foot i { width:5px; height:5px; border-radius:50%; }
.cpu-dot { background:#ed55c6; }.gpu-dot { background:#25c4fa; }.coolant-dot { background:#478aff; }
@media (min-width: 1500px) { .dashboard-page { gap:16px; }.metric-grid,.middle-grid,.bottom-grid { gap:15px; }.metric-card { min-height:228px; }.panel { min-height:322px; }.lcd-preview-frame { height:168px; }.device-panel,.performance-panel { min-height:274px; } }
@media (max-width: 1190px) {
  .middle-grid { grid-template-columns:minmax(0,1.35fr) minmax(230px,.9fr); }
  .lcd-panel { grid-column:1 / -1; min-height:205px; }
  .lcd-preview-frame { height:130px; }
  .bottom-grid { grid-template-columns:minmax(260px,.85fr) minmax(0,1.15fr); }
  .metric-body { gap:0; }.metric-card { padding-inline:11px; }
}
@media (max-width: 850px) {
  .dashboard-page { gap:13px; }
  .metric-grid { grid-template-columns:repeat(2,minmax(0,1fr)); }
  .middle-grid,.bottom-grid { grid-template-columns:1fr 1fr; }
  .fan-panel,.performance-panel { grid-column:1 / -1; }
  .fan-panel { min-height:322px; }
  .fan-chart { height:175px; }
  .lcd-panel { grid-column:auto; }
  .performance-grid { grid-template-columns:repeat(4,minmax(0,1fr)); }
}
@media (max-width: 620px) {
  .welcome-row { align-items:flex-start; flex-direction:column; padding-block:2px 4px; }
  .welcome-tools { width:100%; }.rig-pill { flex:1; }.icon-link { flex:0 0 auto; }
  .middle-grid,.bottom-grid { grid-template-columns:1fr; }
  .fan-panel,.performance-panel,.lcd-panel { grid-column:auto; }
  .rgb-panel { min-height:238px; }.lcd-panel { min-height:243px; }
  .lcd-preview-frame { height:150px; }
  .device-panel { min-height:213px; }
  .performance-grid { grid-template-columns:repeat(2,minmax(0,1fr)); }
  .performance-tile { min-height:92px; }
  .panel-action { font-size:0; }.panel-action svg { width:17px; height:17px; }
}
@media (max-width: 410px) {
  .metric-card { padding:11px 9px; }.metric-body { flex-direction:column; align-items:center; padding-top:6px; }
  .metric-detail { width:100%; padding:0 3px 4px; }.metric-chart { height:31px; }.sensor-caption { display:none; }
  .metric-card { min-height:230px; }.device-online { font-size:0; }.device-online i { width:6px; height:6px; }
}
@media (prefers-reduced-motion: reduce) {
  .progress-track i,.icon-link,.panel-arrow { transition:none; }
}
</style>
