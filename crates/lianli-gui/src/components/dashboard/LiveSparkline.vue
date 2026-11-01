<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { linePath } from "@/utils/visualization";

const props = defineProps<{
  value: number | null;
  sampleTime: number | null;
  tone?: "pink" | "blue" | "cyan" | "violet" | "amber";
}>();
const samples = ref<number[]>([]);
const lastSample = ref<number | null>(null);
watch(() => [props.sampleTime, props.value] as const, ([time, value]) => {
  if (time == null || time === lastSample.value || value == null || !Number.isFinite(value)) return;
  lastSample.value = time;
  samples.value = [...samples.value.slice(-27), value];
}, { immediate: true });
const path = computed(() => linePath(samples.value, 140, 42, 4));
const area = computed(() => path.value ? `${path.value} L 136 40 L 4 40 Z` : "");
const last = computed(() => samples.value[samples.value.length - 1] ?? null);
</script>

<template>
  <svg class="sparkline" :class="`tone-${tone ?? 'blue'}`" viewBox="0 0 140 42" preserveAspectRatio="none" role="img" aria-label="Recent live samples">
    <defs>
      <linearGradient :id="`spark-fill-${tone ?? 'blue'}`" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0%" stop-color="currentColor" stop-opacity=".27" />
        <stop offset="100%" stop-color="currentColor" stop-opacity="0" />
      </linearGradient>
    </defs>
    <path v-if="area" :d="area" :fill="`url(#spark-fill-${tone ?? 'blue'})`" />
    <path v-if="path" :d="path" class="spark-line" />
    <circle v-if="last != null" cx="136" :cy="Number(path.match(/[^ ]+$/)?.[0] ?? 20)" r="2.2" class="spark-dot" />
    <text v-if="!path" x="70" y="24" text-anchor="middle" class="spark-empty">Waiting for samples</text>
  </svg>
</template>

<style scoped>
.sparkline { width: 100%; height: 38px; overflow: visible; color: var(--neon-blue); }
.spark-line { fill: none; stroke: currentColor; stroke-width: 2; stroke-linecap: round; stroke-linejoin: round; filter: drop-shadow(0 0 3px currentColor); }
.spark-dot { fill: currentColor; stroke: rgba(255,255,255,.8); stroke-width: .7; }
.spark-empty { fill: var(--text-muted); font: 8px var(--font-sans); }
.tone-pink { color: var(--neon-pink); }.tone-blue { color: var(--neon-blue); }.tone-cyan { color: var(--neon-cyan); }.tone-violet { color: var(--neon-violet); }.tone-amber { color: var(--neon-amber); }
</style>
