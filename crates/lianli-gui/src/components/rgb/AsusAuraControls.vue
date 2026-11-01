<script setup lang="ts">
import { computed } from "vue";
import { useConfigStore } from "@/stores/config";
import { useDaemonStore } from "@/stores/daemon";
import { useTranslate } from "@/i18n";

const config = useConfigStore();
const daemon = useDaemonStore();
const t = useTranslate();

const aura = computed(() => config.config.asus_aura);
const channelCount = computed(() => daemon.auraLighting?.reported_argb_channels ?? 0);
const channels = computed(() => Array.from({ length: channelCount.value }, (_, index) => index));

function setReversed(channel: number, checked: boolean) {
  const selected = new Set(aura.value.reverse_argb_channels);
  if (checked) selected.add(channel);
  else selected.delete(channel);
  aura.value.reverse_argb_channels = [...selected].sort((a, b) => a - b);
  config.markDirty();
}

function statusLabel() {
  const state = daemon.auraLighting?.worker_state ?? "stopped";
  return t(`rgb.asusAura.states.${state}`);
}
</script>

<template>
  <section class="card aura-card">
    <div class="aura-heading">
      <div>
        <h2>{{ t("rgb.asusAura.title") }}</h2>
        <p class="muted">{{ t("rgb.asusAura.description") }}</p>
      </div>
      <span class="status-tag">
        <span class="status-dot" :class="`state-${daemon.auraLighting?.worker_state ?? 'stopped'}`" />
        {{ statusLabel() }}
      </span>
    </div>
    <div class="aura-controls">
      <n-checkbox
        :checked="aura.rainbow_enabled"
        @update:checked="(value: boolean) => { aura.rainbow_enabled = value; config.markDirty(); }"
      >
        {{ t("rgb.asusAura.enable") }}
      </n-checkbox>
      <div class="channel-directions">
        <span class="muted">{{ t("rgb.asusAura.reverseChannels") }}</span>
        <label v-for="channel in channels" :key="channel" class="channel-choice">
          <input
            type="checkbox"
            :checked="aura.reverse_argb_channels.includes(channel)"
            :disabled="!aura.rainbow_enabled"
            @change="setReversed(channel, ($event.target as HTMLInputElement).checked)"
          >
          {{ t("rgb.asusAura.channel", { channel }) }}
        </label>
        <span v-if="!channelCount" class="muted">{{ t("rgb.asusAura.waitingForChannels") }}</span>
      </div>
    </div>
    <p v-if="daemon.auraLighting?.last_error" class="aura-error">
      {{ daemon.auraLighting.last_error }}
    </p>
  </section>
</template>

<style scoped>
.aura-card {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  padding: var(--space-3) var(--space-4);
}
.aura-heading,
.aura-controls,
.channel-directions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-2) var(--space-4);
}
.aura-heading {
  justify-content: space-between;
}
h2,
p {
  margin: 0;
}
h2 {
  font-size: var(--font-size-md);
}
.aura-heading p,
.channel-directions,
.status-tag {
  font-size: var(--font-size-sm);
}
.channel-directions {
  gap: var(--space-2) var(--space-3);
}
.channel-choice {
  display: inline-flex;
  align-items: center;
  gap: var(--space-1);
}
.status-tag {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  color: var(--text-secondary);
}
.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--text-muted);
}
.state-streaming {
  background: var(--success);
}
.state-connecting,
.state-reconnecting {
  background: var(--warning);
}
.aura-error {
  color: var(--danger);
  font-size: var(--font-size-sm);
  overflow-wrap: anywhere;
}
</style>
