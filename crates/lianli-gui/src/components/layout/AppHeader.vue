<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { useMessage } from "naive-ui";
import { useRoute } from "vue-router";
import { Copy, Minimize, Moon, RefreshCw, Save, Square, Sun, X } from "lucide-vue-next";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useDaemonStore } from "@/stores/daemon";
import { useConfigStore } from "@/stores/config";
import { useThemeStore } from "@/stores/theme";
import { useTranslate } from "@/i18n";
import { MAIN_ROUTES } from "@/router";

const daemon = useDaemonStore();
const config = useConfigStore();
const theme = useThemeStore();
const route = useRoute();
const message = useMessage();
const t = useTranslate();
const currentWindow = getCurrentWindow();
const saving = ref(false);
const maximized = ref(false);

const title = computed(() => {
  const match = MAIN_ROUTES.find((r) => r.name === route.name);
  return t(match?.labelKey ?? "nav.appName");
});

const refreshing = ref(false);
const showGlobalSave = computed(() => !(route.name === "lcd" && config.hasLcdSaveAction));

async function onRefresh() {
  if (refreshing.value) return;
  refreshing.value = true;
  try {
    await daemon.refresh();
  } finally {
    refreshing.value = false;
    // Drop focus so the button doesn't stay in its active/pressed colour.
    (document.activeElement as HTMLElement | null)?.blur();
  }
}

async function onSave() {
  if (saving.value) return;
  saving.value = true;
  try {
    await config.save();
  } catch (error) {
    message.error(t("shell.header.saveFailed", { error: error instanceof Error ? error.message : String(error) }), { duration: 10000 });
  } finally {
    saving.value = false;
    (document.activeElement as HTMLElement | null)?.blur();
  }
}

function onToggleTheme() {
  theme.toggle();
  (document.activeElement as HTMLElement | null)?.blur();
}

async function onMinimize() { await currentWindow.minimize(); }
async function syncMaximized() {
  try {
    maximized.value = await currentWindow.isMaximized();
  } catch (error) {
    console.warn("Unable to read the application window state", error);
  }
}
async function onToggleMaximize() {
  try {
    await currentWindow.toggleMaximize();
    await syncMaximized();
  } catch (error) {
    console.warn("Unable to maximize or restore the application window", error);
  }
}
async function onClose() { await currentWindow.close(); }
function startWindowDrag(event: MouseEvent) {
  if (event.button !== 0 || !(event.target instanceof Element)) return;
  if (event.target.closest("button, a, input, textarea, select, [role='button']")) return;
  event.preventDefault();
  void currentWindow.startDragging().catch((error) => {
    console.warn("Unable to start dragging the application window", error);
  });
}

let unlistenWindowResize: (() => void) | undefined;
onMounted(async () => {
  await syncMaximized();
  unlistenWindowResize = await currentWindow.onResized(() => {
    void syncMaximized();
  });
});
onUnmounted(() => unlistenWindowResize?.());
</script>

<template>
  <header class="header" @mousedown.left="startWindowDrag">
    <h1 v-if="route.name !== 'dashboard'" class="title">{{ title }}</h1>
    <div class="spacer" />
    <n-button
      quaternary
      size="small"
      :title="theme.isDark ? t('shell.header.switchToLight') : t('shell.header.switchToDark')"
      @click="onToggleTheme"
    >
      <template #icon>
        <Sun v-if="theme.isDark" :size="15" />
        <Moon v-else :size="15" />
      </template>
    </n-button>
    <n-button
      quaternary
      size="small"
      :loading="refreshing"
      :disabled="refreshing"
      :title="refreshing ? t('shell.header.refreshing') : t('shell.header.refreshNow')"
      @click="onRefresh"
    >
      <template #icon>
        <RefreshCw :size="15" :class="{ spin: refreshing }" />
      </template>
    </n-button>
    <n-button
      v-if="showGlobalSave"
      type="primary"
      size="small"
      :disabled="saving || !config.dirty || !daemon.canWrite"
      :loading="saving"
      :class="{ dirty: config.dirty }"
      @click="onSave"
    >
      <template #icon><Save :size="15" /></template>
      {{ t("common.save") }}{{ config.dirty ? "*" : "" }}
    </n-button>
    <span v-if="config.loaded && config.dirty" class="save-state pending">{{ t("common.unsavedChanges") }}</span>
    <span v-else-if="config.loaded" class="save-state">{{ t("common.configSaved") }}</span>
    <span class="window-separator" aria-hidden="true" />
    <div class="window-controls">
      <button class="window-button" type="button" :aria-label="t('shell.header.minimizeWindow')" :title="t('shell.header.minimizeWindow')" @click="onMinimize"><Minimize :size="14" /></button>
      <button class="window-button" type="button" :aria-label="t('shell.header.toggleMaximizeWindow')" :title="t('shell.header.toggleMaximizeWindow')" @click="onToggleMaximize">
        <Copy v-if="maximized" :size="12" />
        <Square v-else :size="12" />
      </button>
      <button class="window-button close-button" type="button" :aria-label="t('shell.header.closeWindow')" :title="t('shell.header.closeWindow')" @click="onClose"><X :size="15" /></button>
    </div>
  </header>
</template>

<style scoped>
.header {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  min-height: 42px;
  padding: 0 13px 0 18px;
  border-bottom: 1px solid rgba(140, 177, 227, .13);
  background: linear-gradient(90deg,rgba(24,42,68,.32),rgba(8,19,34,.16));
  flex-shrink: 0;
}
.title {
  color:#d8e8ff;
  font-size: 13px;
  font-weight: 630;
  letter-spacing:.015em;
  margin: 0;
}
.spacer {
  flex: 1;
}
.spin {
  animation: header-spin 0.9s linear infinite;
}
@keyframes header-spin {
  to {
    transform: rotate(360deg);
  }
}
.dirty {
  box-shadow: 0 0 0 1px var(--warning), 0 0 8px rgba(251, 191, 36, 0.4);
}
.save-state {
  color: var(--text-muted);
  font-size: var(--font-size-xs);
  white-space: nowrap;
}
.save-state.pending { color: var(--warning); }
.window-separator { width:1px;height:19px;margin:0 2px 0 4px;background:rgba(150,182,224,.18); }
.window-controls { display:flex;align-items:center;gap:4px; }
.window-button { width:29px;height:27px;display:grid;place-items:center;border:1px solid transparent;border-radius:7px;color:#aabbd2;background:transparent;cursor:pointer;transition:background .14s,color .14s,border-color .14s; }
.window-button:hover { color:#f3f7ff;border-color:rgba(130,176,232,.20);background:rgba(115,151,199,.15); }
.close-button:hover { color:white;border-color:rgba(255,105,132,.36);background:rgba(221,49,83,.78);box-shadow:0 0 15px rgba(235,62,105,.22); }
@media (max-width: 600px) {
  .header { min-height:40px; padding-inline: 8px; gap: var(--space-2); }
  .title { font-size: 12px; }
  .save-state { display: none; }
  .window-button { width:25px; }
}
</style>
