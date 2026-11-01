<script setup lang="ts">
import { useDaemonStore } from "@/stores/daemon";
import { useDevicesStore } from "@/stores/devices";
import { useIpc } from "@/composables/useIpc";
import { computed, ref } from "vue";
import { useMessage } from "naive-ui";
import type { TelemetrySnapshot } from "@/types";
import { useTranslate, type MessageKey } from "@/i18n";
import { backendText } from "@/utils/backendText";
type Preparation = NonNullable<TelemetrySnapshot["media_preparation"]>[string];
const t = useTranslate();
const stateLabels: Record<Preparation["state"], MessageKey> = {
  waiting_for_device: "service.mediaStream.state.waitingForDevice",
  preparing: "service.mediaStream.state.preparing",
  ready: "service.mediaStream.state.ready",
  failed: "service.mediaStream.state.failed",
};
const stageLabels: Record<NonNullable<Preparation["runtime"]>["stage"], MessageKey> = {
  failed: "service.mediaStream.stage.failed",
  starting_source: "service.mediaStream.stage.startingSource",
  autonomous_source_configured: "service.mediaStream.stage.autonomousSourceConfigured",
  frame_submitted: "service.mediaStream.stage.frameSubmitted",
};
const daemon = useDaemonStore();
const devices = useDevicesStore();
const ipc = useIpc();
const message = useMessage();
const retrying = ref(false);
const clearing = ref<string>();
const failed = computed(() => Object.values(daemon.mediaPreparation).some(status => status.state === "failed" || status.runtime?.stage === "failed"));

async function retry() {
  if (retrying.value || !daemon.canWrite) return;
  retrying.value = true;
  try {
    await ipc.request("RetryMedia");
    message.info(t("service.mediaStream.retryQueued"));
    await daemon.refresh();
  } catch (error) {
    message.error(String(error));
  } finally {
    retrying.value = false;
  }
}

async function clearRecovery(deviceId: string) {
  if (clearing.value || !daemon.canWrite) return;
  clearing.value = deviceId;
  try {
    await ipc.request("ClearStartupImageRecovery", { device_id: deviceId });
    message.info(t("service.mediaStream.recoveryReset"));
    await daemon.refresh();
  } catch (error) {
    message.error(String(error));
  } finally {
    clearing.value = undefined;
  }
}
</script>

<template>
  <n-button v-if="failed && daemon.info?.capabilities.includes('media_retry')" class="retry" :loading="retrying" :disabled="retrying || !daemon.canWrite" @click="retry">{{ t("service.mediaStream.retryFailed") }}</n-button>
  <div v-if="Object.keys(daemon.mediaPreparation).length" class="stream-grid">
    <n-card v-for="(status, index) in daemon.mediaPreparation" :key="index" size="small" :title="devices.byId(status.device_id)?.name ?? t('service.mediaStream.lcdFallback', { index: Number(index) + 1 })">
      <template #header-extra><n-tag size="small" :type="status.state === 'failed' || status.runtime?.stage === 'failed' ? 'error' : status.state === 'ready' ? 'success' : 'warning'">{{ status.runtime?.stage === 'failed' ? t("service.mediaStream.stopped") : t(stateLabels[status.state]) }}</n-tag></template>
      <dl>
        <div><dt>{{ t("service.mediaStream.serial") }}</dt><dd>{{ devices.byId(status.device_id)?.serial ?? status.device_id }}</dd></div>
        <div><dt>{{ t("service.mediaStream.transfer") }}</dt><dd>{{ !status.runtime ? '—' : status.runtime.h264_transfer_started != null || status.runtime.encoder || status.runtime.stage === 'autonomous_source_configured' ? 'H.264' : status.runtime.stage === 'frame_submitted' ? 'JPEG' : '—' }}</dd></div>
        <div><dt>{{ t("service.mediaStream.fpsLimit") }}</dt><dd>{{ status.runtime?.fps_limit ?? '—' }}</dd></div>
        <div><dt>{{ t("service.mediaStream.encoder") }}</dt><dd>{{ status.runtime?.encoder?.name ?? '—' }}</dd></div>
      </dl>
      <p v-if="status.error" class="failure">{{ backendText(status.error, status.error_ui) }}</p>
      <n-button v-if="status.startup_recovery_required && daemon.info?.capabilities.includes('startup_recovery_clear')" size="small" :loading="clearing === status.device_id" :disabled="!!clearing || !daemon.canWrite" @click="clearRecovery(status.device_id)">{{ t("service.mediaStream.clearRecovery") }}</n-button>
      <details v-if="status.last_playback_error || status.runtime">
        <summary>{{ t("service.mediaStream.playbackDetails") }}</summary>
        <p v-if="status.runtime">{{ t(status.runtime.hardware_video_allowed ? 'service.mediaStream.stageHardwareEnabled' : 'service.mediaStream.stageHardwareDisabled', { stage: t(stageLabels[status.runtime.stage]) }) }}</p>
        <p v-if="status.runtime?.h264_transfer_started != null">{{ t(status.runtime.h264_transfer_started ? 'service.mediaStream.firstTransferSent' : 'service.mediaStream.firstTransferPending') }}</p>
        <p v-if="status.runtime?.fallback_reason">{{ backendText(status.runtime.fallback_reason, status.runtime.fallback_reason_ui) }}</p>
        <p v-if="status.runtime?.encoder?.software_fallback">{{ t("service.mediaStream.softwareFallback") }}</p>
        <p v-if="status.last_playback_error">{{ t("service.mediaStream.lastError", { error: backendText(status.last_playback_error, status.last_playback_error_ui) }) }}</p>
        <p v-if="status.runtime?.stage === 'failed' || status.state === 'failed'">{{ t(daemon.info?.capabilities.includes('media_retry') ? 'service.mediaStream.repairAndRetry' : 'service.mediaStream.repairAndRestart') }}</p>
      </details>
    </n-card>
  </div>
</template>

<style scoped>
.retry { margin-bottom: var(--space-3); }
.stream-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(min(100%, 320px), 1fr)); gap: var(--space-3); }
dl { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: var(--space-3); margin: 0; }
dt, summary { color: var(--text-secondary); }
dd { margin: 2px 0 0; overflow-wrap: anywhere; }
p { margin: var(--space-2) 0; overflow-wrap: anywhere; }
details { margin-top: var(--space-3); }
summary { cursor: pointer; }
.failure { color: var(--danger); }
</style>
