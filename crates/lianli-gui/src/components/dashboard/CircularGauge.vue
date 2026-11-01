<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(defineProps<{
  value: number | null;
  max?: number;
  tone?: "pink" | "blue" | "cyan" | "violet" | "amber";
  label: string;
  unit?: string;
}>(), { max: 100, tone: "blue", unit: "" });

const percent = computed(() => props.value == null || !Number.isFinite(props.value)
  ? 0
  : Math.max(0, Math.min(100, props.value / props.max * 100)));
const valueText = computed(() => props.value == null || !Number.isFinite(props.value)
  ? "—"
  : `${Math.round(props.value)}${props.unit}`);
</script>

<template>
  <div class="gauge" :class="`tone-${tone}`" role="img" :aria-label="`${label}: ${valueText}`">
    <svg viewBox="0 0 148 148" aria-hidden="true">
      <defs>
        <linearGradient :id="`gauge-${tone}`" x1="0" y1="1" x2="1" y2="0">
          <stop offset="0%" :stop-color="`var(--neon-${tone})`" />
          <stop offset="100%" :stop-color="`var(--neon-${tone}-light)`" />
        </linearGradient>
        <filter :id="`glow-${tone}`" x="-40%" y="-40%" width="180%" height="180%">
          <feGaussianBlur stdDeviation="3.2" result="blur" />
          <feMerge><feMergeNode in="blur" /><feMergeNode in="SourceGraphic" /></feMerge>
        </filter>
      </defs>
      <circle class="gauge-track" cx="74" cy="74" r="61" pathLength="100" />
      <circle
        class="gauge-value"
        :class="`value-${tone}`"
        cx="74" cy="74" r="61" pathLength="100"
        :stroke-dasharray="`${Math.max(0, percent * .76)} 100`"
        :stroke="`url(#gauge-${tone})`"
        :filter="`url(#glow-${tone})`"
      />
      <circle class="gauge-inner" cx="74" cy="74" r="50" />
    </svg>
    <div class="gauge-center">
      <strong>{{ valueText }}</strong>
      <span>{{ label }}</span>
    </div>
  </div>
</template>

<style scoped>
.gauge { width: 142px; height: 142px; position: relative; flex: 0 0 auto; color: var(--neon-blue); }
.gauge svg { width: 100%; height: 100%; overflow: visible; transform: rotate(135deg); }
.gauge-track,.gauge-value { fill: none; stroke-width: 7.5; stroke-linecap: round; }
.gauge-track { stroke: rgba(108, 142, 190, .16); stroke-dasharray: 76 100; }
.gauge-value { transition: stroke-dasharray .55s cubic-bezier(.2,.8,.2,1); }
.gauge-inner { fill: rgba(6, 14, 27, .50); stroke: rgba(190, 217, 255, .10); stroke-width: 1; }
.gauge-center { position: absolute; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 2px; pointer-events: none; }
.gauge-center strong { color: currentColor; font-size: 25px; font-weight: 680; letter-spacing: -.045em; font-variant-numeric: tabular-nums; text-shadow: 0 0 16px color-mix(in srgb, currentColor 52%, transparent); }
.gauge-center span { color: var(--text-muted); font-size: 10px; letter-spacing: .035em; text-transform: uppercase; }
.tone-pink { color: var(--neon-pink); }
.tone-blue { color: var(--neon-blue); }
.tone-cyan { color: var(--neon-cyan); }
.tone-violet { color: var(--neon-violet); }
.tone-amber { color: var(--neon-amber); }
@media (max-width: 1150px) { .gauge { width: 122px; height: 122px; } .gauge-center strong { font-size: 22px; } }
@media (max-width: 720px) { .gauge { width: 110px; height: 110px; } }
</style>
