<script setup lang="ts">
withDefaults(defineProps<{
  as?: string;
  tone?: "blue" | "cyan" | "pink" | "violet" | "amber" | "neutral";
  interactive?: boolean;
}>(), { as: "article", tone: "blue", interactive: false });
</script>

<template>
  <component :is="as" class="glass-card" :class="[`tone-${tone}`, { interactive }]">
    <slot />
  </component>
</template>

<style scoped>
.glass-card {
  --card-tone: var(--neon-blue);
  position: relative;
  min-width: 0;
  overflow: hidden;
  isolation: isolate;
  border: 1px solid color-mix(in srgb, var(--card-tone) 34%, rgba(170, 205, 255, .20));
  border-radius: var(--radius-glass);
  background:
    linear-gradient(145deg, rgba(29, 49, 78, .64), rgba(11, 22, 39, .78) 58%, rgba(11, 20, 34, .88)),
    var(--glass-fallback);
  box-shadow: var(--glass-shadow), 0 0 18px color-mix(in srgb, var(--card-tone) 13%, transparent), inset 0 1px 0 rgba(255,255,255,.09), inset 0 0 34px color-mix(in srgb, var(--card-tone) 11%, transparent);
  -webkit-backdrop-filter: blur(18px) saturate(135%);
  backdrop-filter: blur(18px) saturate(135%);
}
.glass-card::before {
  position: absolute;
  z-index: -1;
  inset: 0;
  border-radius: inherit;
  background:
    radial-gradient(90% 60% at 0% 0%, color-mix(in srgb, var(--card-tone) 17%, transparent), transparent 72%),
    linear-gradient(110deg, rgba(255,255,255,.065), transparent 34% 78%, rgba(255,255,255,.025));
  content: "";
  pointer-events: none;
}
.tone-blue { --card-tone: var(--neon-blue); }
.tone-cyan { --card-tone: var(--neon-cyan); }
.tone-pink { --card-tone: var(--neon-pink); }
.tone-violet { --card-tone: var(--neon-violet); }
.tone-amber { --card-tone: var(--neon-amber); }
.tone-neutral { --card-tone: #8ba7d0; }
.interactive {
  transition: transform .18s ease, border-color .18s ease, box-shadow .18s ease;
}
.interactive:hover {
  transform: translateY(-2px);
  border-color: color-mix(in srgb, var(--card-tone) 62%, white 10%);
  box-shadow: var(--glass-shadow), 0 0 24px color-mix(in srgb, var(--card-tone) 14%, transparent), inset 0 1px 0 rgba(255,255,255,.12);
}
[data-glow="subtle"] .glass-card { box-shadow: 0 12px 34px rgba(0,0,0,.25), inset 0 1px 0 rgba(255,255,255,.08); }
[data-glow="minimal"] .glass-card { -webkit-backdrop-filter: none; backdrop-filter: none; box-shadow: 0 8px 24px rgba(0,0,0,.20); }
@media (prefers-reduced-motion: reduce) {
  .interactive { transition: none; }
  .interactive:hover { transform: none; }
}
</style>
