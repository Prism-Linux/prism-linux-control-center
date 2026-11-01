<script setup lang="ts">
import { computed } from "vue";
import { useDevicesStore } from "@/stores/devices";
import AioCard from "@/components/aio/AioCard.vue";
import { useTranslate } from "@/i18n";

const devices = useDevicesStore();
const t = useTranslate();
const aios = computed(() => devices.aioDevices);
</script>

<template>
  <div class="page aio-page">
    <AioCard v-for="d in aios" :key="d.device_id" :device="d" />
    <div v-if="!aios.length" class="card empty-state muted">
      {{ t("devices.aio.empty") }}
    </div>
  </div>
</template>

<style scoped>
.aio-page {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}
.empty-state {
  padding: var(--space-6);
  text-align: center;
}
</style>
