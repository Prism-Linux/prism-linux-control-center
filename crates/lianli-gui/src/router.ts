import { createRouter, createWebHashHistory, type RouteRecordRaw } from "vue-router";

const routes: RouteRecordRaw[] = [
  { path: "/", redirect: "/dashboard" },
  { path: "/dashboard", name: "dashboard", component: () => import("@/views/DashboardPage.vue") },
  { path: "/devices", name: "devices", component: () => import("@/views/DevicesPage.vue") },
  { path: "/lcd", name: "lcd", component: () => import("@/views/LcdPage.vue") },
  { path: "/fans", name: "fans", component: () => import("@/views/FansPage.vue") },
  { path: "/rgb", name: "rgb", component: () => import("@/views/RgbPage.vue") },
  { path: "/aio", name: "aio", component: () => import("@/views/AioPage.vue") },
  { path: "/settings", name: "settings", component: () => import("@/views/SettingsPage.vue") },
  { path: "/installation", name: "installation", component: () => import("@/views/InstallationPage.vue") },
  // Secondary windows (rendered without the app shell).
  { path: "/editor", name: "editor", component: () => import("@/views/EditorWindow.vue") },
  { path: "/browser", name: "browser", component: () => import("@/views/BrowserWindow.vue") },
];

export const router = createRouter({
  history: createWebHashHistory(),
  routes,
});

export const MAIN_ROUTES = [
  { name: "dashboard", labelKey: "nav.dashboard", icon: "dashboard", to: "/dashboard" },
  { name: "devices", labelKey: "nav.devices", icon: "monitor", to: "/devices" },
  { name: "lcd", labelKey: "nav.lcd", icon: "image", to: "/lcd" },
  { name: "fans", labelKey: "nav.fans", icon: "fan", to: "/fans" },
  { name: "rgb", labelKey: "nav.rgb", icon: "palette", to: "/rgb" },
  { name: "aio", labelKey: "nav.aio", icon: "droplet", to: "/aio" },
  { name: "settings", labelKey: "nav.settings", icon: "settings", to: "/settings" },
  { name: "installation", labelKey: "nav.installation", icon: "health", to: "/installation" },
] as const;
