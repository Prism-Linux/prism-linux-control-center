<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { linePath } from "@/utils/visualization";

export interface FanTrendSeries {
  id: string;
  label: string;
  color: string;
  rpm: number | null;
}
const props = defineProps<{ series: FanTrendSeries[]; sampleTime: number | null }>();
const history = ref<Record<string, number[]>>({});
const lastSample = ref<number | null>(null);
watch(() => [props.sampleTime, props.series.map((entry) => `${entry.id}:${entry.rpm ?? "-"}`).join("|")] as const, ([time]) => {
  if (time == null || time === lastSample.value) return;
  lastSample.value = time;
  const next = { ...history.value };
  for (const item of props.series) {
    if (item.rpm == null || !Number.isFinite(item.rpm) || item.rpm < 0) continue;
    next[item.id] = [...(next[item.id] ?? []).slice(-27), item.rpm];
  }
  history.value = next;
}, { immediate: true });
const maxRpm = computed(() => Math.max(500, ...props.series.flatMap((entry) => history.value[entry.id] ?? [])));
const gridLabels = computed(() => [maxRpm.value, Math.round(maxRpm.value * .66), Math.round(maxRpm.value * .33), 0]);
const lines = computed(() => props.series.map((entry) => ({
  ...entry,
  path: linePath(history.value[entry.id] ?? [], 500, 176, 8, [0, maxRpm.value]),
  lastRpm: history.value[entry.id]?.[history.value[entry.id].length - 1] ?? null,
})).filter((entry) => entry.path));
const pointY = (rpm: number | null) => 168 - ((rpm ?? 0) / maxRpm.value) * 160;
</script>

<template>
  <div class="fan-chart" role="img" aria-label="Recent fan RPM samples">
    <div class="chart-y-axis" aria-hidden="true">
      <span v-for="label in gridLabels" :key="label">{{ label.toLocaleString() }}</span>
    </div>
    <svg viewBox="0 0 500 176" preserveAspectRatio="none" aria-hidden="true">
      <g class="chart-grid"><line v-for="(_,index) in gridLabels" :key="index" x1="0" x2="500" :y1="8 + index * 53" :y2="8 + index * 53" /></g>
      <path v-for="line in lines" :key="line.id" :d="line.path" :style="{ stroke: line.color }" class="trend-line" />
      <circle v-for="line in lines" :key="`${line.id}-point`" cx="492" :cy="pointY(line.lastRpm)" r="3.4" :style="{ fill: line.color }" class="trend-point" />
      <text v-if="!lines.length" x="250" y="94" text-anchor="middle" class="chart-empty">Collecting live fan samples</text>
    </svg>
    <div class="chart-x-axis" aria-hidden="true"><span>Live</span><span>{{ sampleTime ? new Date(sampleTime).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }) : "—" }}</span></div>
  </div>
</template>

<style scoped>
.fan-chart { position: relative; display: grid; grid-template-columns: 37px minmax(0,1fr); grid-template-rows: minmax(0,1fr) 18px; width: 100%; height: 162px; margin-top: 10px; }
.fan-chart svg { width: 100%; height: 100%; overflow: visible; grid-column: 2; grid-row: 1; }
.chart-grid line { stroke: rgba(129,164,210,.13); stroke-width: 1; stroke-dasharray: 2 5; }
.trend-line { fill: none; stroke-width: 2.8; stroke-linecap: round; stroke-linejoin: round; filter: drop-shadow(0 0 3px currentColor); }
.trend-point { stroke: rgba(255,255,255,.8); stroke-width:1; filter:drop-shadow(0 0 5px currentColor); }
.chart-y-axis { grid-column: 1; grid-row: 1; display: flex; flex-direction: column; justify-content: space-between; align-items: flex-end; padding: 1px 7px 0 0; color: var(--text-muted); font: 9px var(--font-mono); }
.chart-x-axis { grid-column: 2; grid-row: 2; display: flex; justify-content: space-between; color: var(--text-muted); font: 9px var(--font-mono); padding-top: 4px; }
.chart-empty { fill: var(--text-muted); font: 11px var(--font-sans); }
</style>
