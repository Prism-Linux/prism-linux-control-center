<script setup lang="ts">
import { ref, onMounted } from "vue";
import { RouterLink } from "vue-router";
import { invoke } from "@/utils/backendText";
import * as icons from "lucide-vue-next";
import { useTranslate } from "@/i18n";
import { MAIN_ROUTES } from "@/router";
import { useDaemonStore } from "@/stores/daemon";
import { useDevicesStore } from "@/stores/devices";

const daemon = useDaemonStore();
const devices = useDevicesStore();
const t = useTranslate();
const APP_VERSION = ref("...");

onMounted(async () => {
  APP_VERSION.value = await invoke<string>("app_version");
});

// Map icon name strings from the route table to Lucide components.
function iconFor(name: string) {
  const map: Record<string, any> = {
    monitor: icons.Monitor,
    dashboard: icons.LayoutDashboard,
    image: icons.Image,
    fan: icons.Fan,
    palette: icons.Palette,
    droplet: icons.Droplet,
    settings: icons.Settings,
    health: icons.HeartPulse,
  };
  return map[name] ?? icons.Circle;
}
</script>

<template>
  <aside class="sidebar">
    <div class="brand">
      <div class="brand-mark"><icons.Sparkles :size="18" /></div>
      <div class="brand-text"><div class="brand-title">Prism Linux Control Center</div><div class="brand-subtitle">Linux hardware control · v{{ APP_VERSION }}</div></div>
    </div>

    <nav class="nav">
      <RouterLink
        v-for="r in MAIN_ROUTES"
        :key="r.name"
        :to="r.to"
        class="nav-item"
        active-class="active"
      >
        <component :is="iconFor(r.icon)" :size="18" :stroke-width="2" />
        <span>{{ t(r.labelKey) }}</span>
      </RouterLink>
    </nav>

    <div class="footer">
      <div class="conn" :class="{ ok: daemon.connected, bad: !daemon.connected }">
        <span class="dot" />
        {{ daemon.connected ? t("shell.sidebar.connected") : t("common.offline") }}
      </div>
      <div class="device-count">{{ daemon.connected ? devices.summary : t("shell.sidebar.lastSeen", { summary: devices.summary }) }}</div>
    </div>
  </aside>
</template>

<style scoped>
.sidebar {
  width: 218px;
  flex-shrink: 0;
  position:relative;
  background:linear-gradient(180deg,rgba(14,26,46,.93),rgba(8,18,33,.90));
  border-right: 1px solid rgba(129,171,229,.22);
  display: flex;
  flex-direction: column;
  padding: 19px 12px 12px;
  box-shadow:inset -1px 0 0 rgba(255,255,255,.025),inset 0 0 26px rgba(79,135,217,.04);
}
.sidebar::before{position:absolute;inset:0 0 auto;height:1px;content:"";background:linear-gradient(90deg,rgba(238,116,255,.78),rgba(71,159,255,.42),transparent)}
.brand {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 7px 7px 23px;
}
.brand-mark{width:42px;height:42px;display:grid;place-items:center;border-radius:14px;color:#d2e6ff;background:linear-gradient(140deg,rgba(59,145,255,.32),rgba(176,78,255,.25));border:1px solid rgba(135,182,255,.37);box-shadow:0 0 20px rgba(82,132,255,.16),inset 0 1px rgba(255,255,255,.15)}
.brand-title {
  font-weight: 700;
  font-size: 13px;
  line-height:1.22;
  letter-spacing: -.02em;
}
.brand-subtitle {
  margin-top:3px;
  font-size: 8px;
  letter-spacing:.035em;
  color: var(--text-muted);
}
.nav {
  display: flex;
  flex-direction: column;
  gap: 5px;
  margin-top: 3px;
}
.nav-item {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  min-height:43px;
  padding: 10px 12px;
  border:1px solid transparent;
  border-radius: 12px;
  color: #9aacc7;
  font-size: 12px;
  cursor: pointer;
  transition: background .16s, color .16s, transform .16s, border-color .16s, box-shadow .16s;
}
.nav-item:hover {
  background:rgba(68,107,158,.18);
  border-color:rgba(131,175,233,.14);
  color:#e7f2ff;
}
.nav-item.active {
  position:relative;
  color:#f0eaff;
  border-color:rgba(151,141,255,.36);
  background:linear-gradient(100deg,rgba(156,58,217,.38),rgba(45,117,255,.35) 74%,rgba(35,187,255,.24));
  box-shadow:inset 0 1px rgba(255,255,255,.13),inset 0 0 17px rgba(151,105,255,.16),0 0 18px rgba(94,93,255,.18);
}
.nav-item.active::after{position:absolute;inset:0;border-radius:inherit;content:"";pointer-events:none;box-shadow:inset 0 0 0 1px rgba(255,255,255,.05)}
@media(max-width:820px){.sidebar{width:68px;padding:14px 8px}.brand{justify-content:center;padding:5px 0 18px}.brand>div:last-child,.nav-item span,.device-count{display:none}.nav-item{justify-content:center;padding:11px}.footer{display:flex;justify-content:center}.conn{font-size:0}.conn .dot{width:9px;height:9px}}
.footer {
  margin-top: auto;
  padding: 12px 10px;
  border:1px solid rgba(125,169,227,.19);
  border-radius:13px;
  background:linear-gradient(145deg,rgba(22,43,70,.52),rgba(9,20,36,.72));
  box-shadow:inset 0 1px rgba(255,255,255,.05);
}
.conn {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  font-size: 10px;
}
.conn .dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}
.conn.ok .dot {
  background: var(--success);
  box-shadow: 0 0 6px var(--success);
}
.conn.bad .dot {
  background: var(--danger);
}
.device-count {
  font-size: 9px;
  color: var(--text-muted);
  margin-top: var(--space-1);
}
</style>
