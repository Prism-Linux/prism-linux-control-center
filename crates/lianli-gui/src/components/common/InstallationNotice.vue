<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useRouter } from "vue-router";
import { useExternalLink } from "@/composables/useExternalLink";
import { useInstallationStore } from "@/stores/installation";
import { useDaemonStore } from "@/stores/daemon";
import { INSTALLATION_GUIDES } from "@/types/installation";
import { canSetUpServices } from "@/utils/serviceSetup";
import { findingRemediation, findingTitle } from "@/utils/findingText";
import { useTranslate } from "@/i18n";
import ServiceStatus from "./ServiceStatus.vue";

const t = useTranslate();
const installation = useInstallationStore();
const daemon = useDaemonStore();
const router = useRouter();
const initialSetup = computed(() => !daemon.connected && canSetUpServices(installation.report?.services));
onMounted(() => { void installation.start(); });

const openExternal = useExternalLink();
async function openGuide() {
  const fallback = installation.report?.context.kind === "distrobox" ? "distrobox" : "service_modes";
  await openExternal(INSTALLATION_GUIDES[installation.issues[0]?.guide ?? fallback]);
}

function review() {
  installation.popupOpen = false;
  void router.push("/installation");
}

async function recheck() {
  await Promise.all([installation.recheck(), daemon.refresh()]);
}
</script>

<template>
  <n-modal v-model:show="installation.popupOpen" preset="card" :title="t('shell.installationNotice.title')"
    style="width: min(580px, 92vw)" :content-style="{ maxHeight: '65vh', overflowY: 'auto' }">
    <p>{{ t("shell.installationNotice.intro") }}</p>
    <n-alert v-if="installation.error" type="warning">{{ installation.error }}</n-alert>
    <ServiceStatus v-if="initialSetup" setup-only />
    <n-alert v-else-if="!daemon.connected" type="warning" :title="t('shell.installationNotice.waitingTitle')">
      {{ t("shell.installationNotice.waitingBody") }}
    </n-alert>
    <div v-for="finding in installation.issues" :key="finding.code" class="notice-finding">
      <strong>{{ findingTitle(finding) }}</strong>
      <p>{{ finding.evidence }}</p>
      <p>{{ findingRemediation(finding) }}</p>
    </div>
    <p v-if="daemon.connected && !installation.error && !installation.issues.length">{{ t("shell.installationNotice.allPass") }}</p>
    <template #footer>
      <n-space justify="end" wrap>
        <n-button @click="installation.popupOpen = false">{{ t("shell.installationNotice.later") }}</n-button>
        <n-button :loading="installation.checking" @click="recheck">{{ t("shell.installationNotice.recheck") }}</n-button>
        <n-button @click="openGuide">{{ t("shell.installationNotice.openGuide") }}</n-button>
        <n-button type="primary" @click="review">{{ t("shell.installationNotice.reviewChecks") }}</n-button>
      </n-space>
    </template>
  </n-modal>
</template>

<style scoped>
.notice-finding { margin-top: var(--space-4); overflow-wrap: anywhere; }
p { margin: var(--space-2) 0; }
</style>
