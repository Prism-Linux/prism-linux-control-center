<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useExternalLink } from "@/composables/useExternalLink";
import { useDaemonStore } from "@/stores/daemon";
import { useInstallationStore } from "@/stores/installation";
import MediaStreamStatus from "@/components/common/MediaStreamStatus.vue";
import DiagnosticExport from "@/components/common/DiagnosticExport.vue";
import DesktopStreamStatus from "@/components/common/DesktopStreamStatus.vue";
import { INSTALLATION_GUIDES, type CheckState, type InstallationGuide } from "@/types/installation";
import { DAEMON_MODE_LABELS } from "@/utils/serviceSetup";
import { findingContext, findingFeature, findingRemediation, findingTitle } from "@/utils/findingText";
import { useTranslate, type MessageKey } from "@/i18n";

const t = useTranslate();
const installation = useInstallationStore();
const daemon = useDaemonStore();
const checkedAt = computed(() => installation.report
  ? new Date(installation.report.checked_at_unix_ms).toLocaleString() : t("service.installation.notChecked"));
const needsAttention = computed(() => (installation.report?.findings ?? []).filter((finding) => finding.state !== "passed" && finding.state !== "not_applicable"));
const otherFindings = computed(() => (installation.report?.findings ?? []).filter((finding) => finding.state === "passed" || finding.state === "not_applicable"));
const stateLabels: Record<CheckState, MessageKey> = {
  passed: "service.installation.state.passed",
  failed: "service.installation.state.failed",
  unavailable: "service.installation.state.unavailable",
  not_applicable: "service.installation.state.notApplicable",
};
onMounted(() => {
  if (!installation.report) void installation.recheck();
});

const openExternal = useExternalLink();
async function guide(id: InstallationGuide) {
  await openExternal(INSTALLATION_GUIDES[id]);
}

async function recheck() {
  await Promise.all([installation.recheck(), daemon.refresh()]);
}
</script>

<template>
  <div class="health-page">
    <div class="health-header">
      <div><h1>{{ t("service.installation.title") }}</h1><p>{{ installation.contextLabel }} · {{ checkedAt }}</p></div>
      <n-button :loading="installation.checking" @click="recheck">{{ t("service.installation.recheck") }}</n-button>
    </div>
    <n-alert v-if="installation.error" type="error" :title="t('service.installation.checkFailedTitle')">
      {{ t("service.installation.checkFailed", { error: installation.error }) }}
    </n-alert>
    <div class="connection-row">
      <n-tag :type="daemon.connected ? 'success' : 'warning'">{{ t(daemon.connected ? 'service.installation.connected' : 'service.installation.offline') }}</n-tag>
      <span v-if="daemon.info">{{ daemon.info.version }} · {{ t(DAEMON_MODE_LABELS[daemon.info.mode]) }}</span>
      <router-link to="/settings">{{ t("service.installation.serviceSettings") }}</router-link>
      <n-button size="small" text type="primary" @click="guide(installation.report?.context.kind === 'distrobox' ? 'distrobox' : 'service_modes')">{{ t("service.installation.setupGuide") }}</n-button>
    </div>
    <p v-if="!daemon.connected" class="muted">{{ t("service.installation.startServiceHint") }}</p>
    <div v-if="needsAttention.length" class="check-list">
      <details v-for="finding in needsAttention" :key="finding.code" class="check-row">
        <summary>
          <n-tag size="small" :type="finding.severity === 'error' ? 'error' : 'warning'">{{ t(stateLabels[finding.state]) }}</n-tag>
          <span class="check-text">
            <span class="check-title">{{ findingTitle(finding) }}</span>
            <span class="check-sub">{{ findingRemediation(finding) || finding.evidence }}</span>
          </span>
        </summary>
        <div class="check-details">
          <p class="check-context">{{ findingContext(finding) }} · {{ findingFeature(finding) }}</p>
          <p>{{ finding.evidence }}</p>
          <n-button size="small" text type="primary" @click="guide(finding.guide)">{{ t("service.installation.openGuide") }}</n-button>
        </div>
      </details>
    </div>
    <p v-else-if="installation.report" class="all-good">{{ t("service.installation.allGood") }}</p>
    <details v-if="otherFindings.length" class="passed-checks">
      <summary>{{ t("service.installation.otherChecks", { count: otherFindings.length }) }}</summary>
      <div class="check-list">
        <details v-for="finding in otherFindings" :key="finding.code" class="check-row">
          <summary>
            <n-tag size="small" :type="finding.state === 'passed' ? 'success' : 'default'">{{ t(stateLabels[finding.state]) }}</n-tag>
            <span class="check-text">
              <span class="check-title">{{ findingTitle(finding) }}</span>
              <span class="check-sub">{{ findingFeature(finding) }}</span>
            </span>
          </summary>
          <div class="check-details">
            <p>{{ finding.evidence }}</p>
            <n-button size="small" text type="primary" @click="guide(finding.guide)">{{ t("service.installation.openGuide") }}</n-button>
          </div>
        </details>
      </div>
    </details>
    <MediaStreamStatus />
    <DesktopStreamStatus />
    <details class="diagnostics"><summary>{{ t("service.installation.exportDiagnostics") }}</summary><DiagnosticExport /></details>
    <details><summary>{{ t("service.installation.aboutChecks") }}</summary><p>{{ t("service.installation.aboutChecksText") }}</p><p v-if="daemon.socketPath">{{ daemon.socketPath }}</p></details>
  </div>
</template>

<style scoped>
.health-page { display: flex; flex-direction: column; gap: var(--space-4); }
.health-header { display: flex; align-items: center; justify-content: space-between; gap: var(--space-4); }
h1 { font-size: var(--font-size-2xl); margin: 0; }
p { margin: var(--space-3) 0; overflow-wrap: anywhere; }
.check-context { color: var(--text-secondary); }
.check-list { border: 1px solid var(--border); border-radius: var(--radius-md); background: var(--bg-surface); }
.check-row + .check-row { border-top: 1px solid var(--border); }
.check-row > summary { display: flex; align-items: flex-start; gap: var(--space-3); margin: 0; padding: var(--space-2) var(--space-3); color: var(--text-primary); list-style: none; }
.check-row > summary::-webkit-details-marker { display: none; }
.check-row > summary:hover { background: var(--bg-hover); }
.check-row > summary .n-tag { flex-shrink: 0; width: 82px; justify-content: center; }
.check-text { display: flex; flex-direction: column; min-width: 0; }
.check-title { font-weight: 600; font-size: var(--font-size-sm); }
.check-sub { font-size: var(--font-size-xs); color: var(--text-secondary); overflow-wrap: anywhere; }
.check-row:not([open]) .check-sub { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.check-details { padding: 0 var(--space-3) var(--space-2) calc(82px + var(--space-3) * 2); font-size: var(--font-size-sm); }
.check-details p { margin: var(--space-1) 0; }
.passed-checks > .check-list { margin-top: var(--space-2); }
.all-good { color: var(--success); margin: 0; }
.connection-row { display: flex; flex-wrap: wrap; align-items: center; gap: var(--space-3); }
summary { cursor: pointer; color: var(--text-secondary); margin-bottom: var(--space-2); }
</style>
