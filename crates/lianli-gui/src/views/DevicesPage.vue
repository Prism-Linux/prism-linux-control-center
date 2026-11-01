<script setup lang="ts">
import { computed } from "vue";
import { useDevicesStore, type DisplaySwitchError } from "@/stores/devices";
import DeviceCard from "@/components/devices/DeviceCard.vue";
import { PlugZap, RefreshCw } from "lucide-vue-next";
import { useDaemonStore } from "@/stores/daemon";
import { useTranslate } from "@/i18n";

const devices = useDevicesStore();
const daemon = useDaemonStore();
const t = useTranslate();

const empty = computed(() => devices.displayCards.length === 0);

function switchErrorText(error: DisplaySwitchError): string {
  return "message" in error ? error.message : t("devices.page.switchTimeout", { name: error.timedOutDevice });
}

async function refresh() {
  await daemon.refresh();
}
</script>

<template>
  <div class="page devices-page">
    <div class="page-head">
      <span class="muted">{{ devices.summary }}</span>
      <n-button quaternary size="small" @click="refresh">
        <template #icon><RefreshCw :size="15" /></template>
      </n-button>
    </div>

    <n-alert v-for="(error, id) in devices.displaySwitchErrors" :key="id" type="error" closable @close="delete devices.displaySwitchErrors[id]">{{ switchErrorText(error) }}</n-alert>

    <div v-if="empty" class="empty-state">
      <PlugZap :size="48" :stroke-width="1.4" />
      <div class="empty-title">{{ t("devices.page.noDevicesFound") }}</div>
      <div class="empty-sub">{{ t("devices.page.daemonRunning") }}</div>
      <n-button size="small" @click="refresh">{{ t("common.retry") }}</n-button>
    </div>

    <div v-else class="card device-list">
      <DeviceCard
        v-for="d in devices.displayCards"
        :key="d.device_id"
        :device="d"
      />
    </div>
  </div>
</template>

<style scoped>
.page {
  width: 100%;
}
.page-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: var(--space-4);
}
.device-list {
  container-type: inline-size;
  display: grid;
  grid-template-columns: 36px minmax(250px, 1.35fr) 130px minmax(0, 1.2fr) auto;
  column-gap: var(--space-3);
  padding: var(--space-1) var(--space-3);
}
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--space-2);
  padding: var(--space-8);
  color: var(--text-muted);
}
.empty-title {
  font-size: var(--font-size-lg);
  color: var(--text-secondary);
  margin-top: var(--space-2);
}
.empty-sub {
  font-size: var(--font-size-sm);
  margin-bottom: var(--space-2);
}
</style>
