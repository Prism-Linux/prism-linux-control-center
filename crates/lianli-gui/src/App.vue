<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { darkTheme, dateZhCN, zhCN, type GlobalTheme, type GlobalThemeOverrides } from "naive-ui";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useDaemonStore } from "@/stores/daemon";
import { useConfigStore } from "@/stores/config";
import { useThemeStore } from "@/stores/theme";
import { useLanguageStore } from "@/stores/language";
import { LCD_TEMPLATES_CHANGED_EVENT } from "@/stores/lcd";
import AppSidebar from "@/components/layout/AppSidebar.vue";
import AppHeader from "@/components/layout/AppHeader.vue";
import InstallationNotice from "@/components/common/InstallationNotice.vue";
import CompatibilityNotice from "@/components/common/CompatibilityNotice.vue";
import WirelessNotice from "@/components/common/WirelessNotice.vue";
import UpdateNotice from "@/components/common/UpdateNotice.vue";
import PageErrorBoundary from "@/components/common/PageErrorBoundary.vue";

const daemon = useDaemonStore();
const config = useConfigStore();
const theme = useThemeStore();
const currentWindow = getCurrentWindow();
const language = useLanguageStore();
const route = useRoute();
const router = useRouter();
const naiveLocale = computed(() => (language.locale === "zh-CN" ? zhCN : null));
const naiveDateLocale = computed(() => (language.locale === "zh-CN" ? dateZhCN : null));

// Secondary windows (editor/browser) render standalone, without the shell.
const isSecondaryWindow = computed(
  () => route.name === "editor" || route.name === "browser",
);

const compactAlert: GlobalThemeOverrides["Alert"] = {
  padding: "8px",
  iconSize: "20px",
  iconMargin: "8px 8px 0 12px",
  closeMargin: "8px 10px 0 0",
  lineHeight: "1.5",
};

const darkOverrides: GlobalThemeOverrides = {
  Alert: compactAlert,
  common: {
    bodyColor: "#07101d",
    cardColor: "rgba(13, 25, 43, .94)",
    modalColor: "rgba(12, 24, 42, .98)",
    popoverColor: "rgba(22, 39, 63, .98)",
    primaryColor: "#4f9eff",
    primaryColorHover: "#3a8be5",
    primaryColorPressed: "#2f7acc",
    textColorBase: "#edf4ff",
    textColor1: "#edf4ff",
    textColor2: "#a6b7d0",
    textColor3: "#7588a4",
    placeholderColor: "#6b7280",
    dividerColor: "rgba(132, 169, 218, .18)",
    borderColor: "rgba(132, 169, 218, .24)",
    inputColor: "rgba(8, 19, 34, .82)",
    inputColorDisabled: "rgba(12, 24, 42, .62)",
    actionColor: "rgba(31, 55, 86, .62)",
    tableHeaderColor: "rgba(11, 23, 40, .86)",
    hoverColor: "rgba(66, 140, 255, 0.15)",
    borderRadius: "10px",
    borderRadiusSmall: "8px",
  },
};

const lightOverrides: GlobalThemeOverrides = {
  Alert: compactAlert,
  common: {
    bodyColor: "#f4f6f9",
    cardColor: "#ffffff",
    modalColor: "#ffffff",
    popoverColor: "#ffffff",
    primaryColor: "#2f7acc",
    primaryColorHover: "#2569b0",
    primaryColorPressed: "#1d5499",
    textColorBase: "#1a1d27",
    textColor1: "#1a1d27",
    textColor2: "#3d4452",
    textColor3: "#565d6b",
    placeholderColor: "#868f9e",
    dividerColor: "#e2e6ee",
    borderColor: "#e2e6ee",
    inputColor: "#ffffff",
    inputColorDisabled: "#eef1f6",
    actionColor: "#eef1f6",
    tableHeaderColor: "#f4f6f9",
    hoverColor: "rgba(47, 122, 204, 0.12)",
    borderRadius: "8px",
    borderRadiusSmall: "6px",
  },
};

const naiveTheme = computed<GlobalTheme | null>(() =>
  theme.isDark ? darkTheme : null,
);
const themeOverrides = computed(() =>
  theme.isDark ? darkOverrides : lightOverrides,
);

type ResizeDirection = "East" | "North" | "NorthEast" | "NorthWest" | "South" | "SouthEast" | "SouthWest" | "West";
function startWindowResize(event: MouseEvent, direction: ResizeDirection) {
  if (event.button !== 0) return;
  event.preventDefault();
  void currentWindow.startResizeDragging(direction).catch((error) => {
    console.warn("Unable to start resizing the application window", error);
  });
}

let unlistenTemplatesChanged: UnlistenFn | undefined;
let unlistenTrayNavigation: UnlistenFn | undefined;

onMounted(async () => {
  // Kick off the initial config load + polling loop.
  await config.load().catch(() => undefined);
  await invoke("set_close_to_tray", { enabled: config.config.desktop.close_to_tray }).catch(() => undefined);
  if (currentWindow.label === "main") {
    unlistenTrayNavigation = await listen<string>("tray:navigate", (event) => {
      void router.push(event.payload);
    });
  }
  daemon.start();

  unlistenTemplatesChanged = await listen(LCD_TEMPLATES_CHANGED_EVENT, () => {
    config.refreshTemplates().catch(() => undefined);
  });
});

onUnmounted(() => {
  unlistenTemplatesChanged?.();
  unlistenTrayNavigation?.();
});
</script>

<template>
  <n-config-provider
    :theme="naiveTheme"
    :theme-overrides="themeOverrides"
    :locale="naiveLocale"
    :date-locale="naiveDateLocale"
  >
    <n-message-provider>
      <n-dialog-provider>
        <n-notification-provider>
          <template v-if="isSecondaryWindow">
            <CompatibilityNotice />
            <PageErrorBoundary>
              <router-view v-slot="{ Component }">
                <transition name="page" mode="out-in">
                  <component :is="Component" />
                </transition>
              </router-view>
            </PageErrorBoundary>
          </template>
          <template v-else>
            <InstallationNotice />
            <div class="window-frame">
              <div class="shell">
                <AppSidebar />
                <div class="main">
                  <AppHeader />
                  <div class="content">
                    <div class="notices">
                      <CompatibilityNotice />
                      <WirelessNotice />
                      <UpdateNotice />
                    </div>
                    <PageErrorBoundary>
                      <router-view v-slot="{ Component }">
                        <transition name="page" mode="out-in">
                          <component :is="Component" />
                        </transition>
                      </router-view>
                    </PageErrorBoundary>
                  </div>
                </div>
              </div>
              <div class="resize-layer" aria-hidden="true">
                <span class="resize-edge resize-n" @mousedown.left="startWindowResize($event, 'North')" />
                <span class="resize-edge resize-s" @mousedown.left="startWindowResize($event, 'South')" />
                <span class="resize-edge resize-e" @mousedown.left="startWindowResize($event, 'East')" />
                <span class="resize-edge resize-w" @mousedown.left="startWindowResize($event, 'West')" />
                <span class="resize-corner resize-ne" @mousedown.left="startWindowResize($event, 'NorthEast')" />
                <span class="resize-corner resize-nw" @mousedown.left="startWindowResize($event, 'NorthWest')" />
                <span class="resize-corner resize-se" @mousedown.left="startWindowResize($event, 'SouthEast')" />
                <span class="resize-corner resize-sw" @mousedown.left="startWindowResize($event, 'SouthWest')" />
              </div>
            </div>
          </template>
        </n-notification-provider>
      </n-dialog-provider>
    </n-message-provider>
  </n-config-provider>
</template>

<style scoped>
.shell {
  display: flex;
  position: relative;
  height: calc(100vh - 42px);
  width: calc(100vw - 42px);
  margin: 21px;
  overflow: hidden;
  border: 1px solid rgba(105, 164, 247, .42);
  border-radius: 24px;
  background:
    radial-gradient(720px 360px at 70% -8%, rgba(51, 104, 174, .19), transparent 73%),
    radial-gradient(560px 400px at 104% 78%, rgba(20, 137, 226, .08), transparent 72%),
    linear-gradient(125deg, rgba(8, 17, 31, .91), rgba(7, 17, 31, .80) 58%, rgba(8, 18, 32, .92));
  box-shadow: 0 24px 85px rgba(0,0,0,.56), 0 0 0 1px rgba(213,126,255,.22), 0 0 30px rgba(40,126,255,.34), 0 0 58px rgba(172,61,255,.18), inset 0 1px 0 rgba(255,255,255,.14);
}
.window-frame { position:relative; flex:1; min-width:0; min-height:0; overflow:hidden; }
.resize-layer { position:absolute; z-index:20; inset:0; pointer-events:none; }
.resize-edge,.resize-corner { position:absolute; display:block; pointer-events:auto; }
.resize-n,.resize-s { left:9px; right:9px; height:7px; }
.resize-n { top:0; cursor:n-resize; }
.resize-s { bottom:0; cursor:s-resize; }
.resize-e,.resize-w { top:9px; bottom:9px; width:7px; }
.resize-e { right:0; cursor:e-resize; }
.resize-w { left:0; cursor:w-resize; }
.resize-corner { width:13px; height:13px; }
.resize-ne { top:0; right:0; cursor:ne-resize; }
.resize-nw { top:0; left:0; cursor:nw-resize; }
.resize-se { bottom:0; right:0; cursor:se-resize; }
.resize-sw { bottom:0; left:0; cursor:sw-resize; }
.shell::before { position:absolute; z-index:0; inset:0; border-radius:inherit; content:""; pointer-events:none; box-shadow:inset 0 0 34px rgba(69,140,248,.06),inset 0 0 0 1px rgba(255,255,255,.025); }
.shell > * { position:relative; z-index:1; }
.main {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
}
.content {
  flex: 1;
  overflow-y: auto;
  padding: 14px 21px 20px;
  scrollbar-gutter: stable;
}
@media(max-width:900px){.shell{height:calc(100vh - 24px);width:calc(100vw - 24px);margin:12px;border-radius:20px}.content{padding:13px 15px 18px}}
@media(max-width:560px){.shell{height:100vh;width:100vw;margin:0;border-radius:0;border:0}.content{padding:12px}}
.notices {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.notices:not(:empty) {
  margin-bottom: var(--space-4);
}
</style>
