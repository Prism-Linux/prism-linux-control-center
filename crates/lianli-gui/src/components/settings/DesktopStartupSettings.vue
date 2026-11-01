<script setup lang="ts">
import { onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useConfigStore } from "@/stores/config";
import { useTranslate } from "@/i18n";

const config = useConfigStore();
const t = useTranslate();
const trayAvailable = ref(false);
const session = ref("");

onMounted(async () => {
  trayAvailable.value = await invoke<boolean>("tray_available").catch(() => false);
  session.value = await invoke<string>("desktop_session").catch(() => t("settings.desktop.unknownSession"));
});

function update(key: "start_with_linux" | "launch_minimized" | "close_to_tray", value: boolean) {
  config.config.desktop[key] = value;
  config.markDirty();
}
</script>

<template>
  <section class="settings-card">
    <div class="heading">
      <div>
        <h2>{{ t("settings.desktop.title") }}</h2>
        <p class="muted">{{ t("settings.desktop.session", { session }) }}</p>
      </div>
    </div>
    <div class="options">
      <n-checkbox
        :checked="config.config.desktop.start_with_linux"
        @update:checked="(value: boolean) => update('start_with_linux', value)"
      >
        {{ t("settings.desktop.startWithLinux") }}
      </n-checkbox>
      <n-checkbox
        :checked="config.config.desktop.launch_minimized"
        :disabled="!trayAvailable"
        @update:checked="(value: boolean) => update('launch_minimized', value)"
      >
        {{ t("settings.desktop.launchMinimized") }}
      </n-checkbox>
      <n-checkbox
        :checked="config.config.desktop.close_to_tray"
        :disabled="!trayAvailable"
        @update:checked="(value: boolean) => update('close_to_tray', value)"
      >
        {{ t("settings.desktop.closeToTray") }}
      </n-checkbox>
    </div>
    <p v-if="!trayAvailable" class="hint">{{ t("settings.desktop.trayUnavailable") }}</p>
    <p v-else class="hint">{{ t("settings.desktop.trayHint") }}</p>
    <p class="hint">{{ t("settings.desktop.daemonHint") }}</p>
    <n-alert v-if="config.desktopStartupError" type="warning" :show-icon="false">
      {{ t("settings.desktop.autostartError", { error: config.desktopStartupError }) }}
    </n-alert>
  </section>
</template>

<style scoped>
.settings-card {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  padding: var(--space-4);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  background: var(--card);
}
.heading h2,
.heading p,
.hint {
  margin: 0;
}
.heading h2 {
  font-size: var(--font-size-md);
}
.heading p,
.hint {
  font-size: var(--font-size-sm);
}
.options {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-3) var(--space-5);
}
</style>
