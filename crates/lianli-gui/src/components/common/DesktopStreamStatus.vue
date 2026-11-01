<script setup lang="ts">
import { useDaemonStore } from "@/stores/daemon";
import type { DesktopStreamStatus } from "@/types";
import { ref } from "vue";
import { useMessage } from "naive-ui";
import { useIpc } from "@/composables/useIpc";
import { useTranslate, type MessageKey } from "@/i18n";
import { backendText } from "@/utils/backendText";

const t = useTranslate();
const daemon = useDaemonStore();
const ipc = useIpc();
const message = useMessage();
const retrying = ref(false);
async function retry(stream: DesktopStreamStatus) {
  if (retrying.value) return;
  retrying.value = true;
  try {
    await ipc.request("RetryDesktopDisplay", { bus: stream.bus, address: stream.address, product_id: stream.product_id });
    message.info(t("service.desktopStream.retryRequested"));
    await daemon.refresh();
  } catch (error) {
    message.error(String(error));
  } finally { retrying.value = false; }
}
const labels: Record<DesktopStreamStatus["state"], MessageKey> = {
  waiting_for_session: "service.desktopStream.state.waitingForSession",
  starting: "service.desktopStream.state.starting",
  streaming: "service.desktopStream.state.streaming",
  paused: "service.desktopStream.state.paused",
  failed: "service.desktopStream.state.failed",
};
</script>

<template>
  <div v-if="daemon.desktopStreams.length" class="stream-grid">
    <n-card v-for="stream in daemon.desktopStreams" :key="`${stream.bus}:${stream.address}`" size="small"
      :title="t('service.desktopStream.title', { product: stream.product_id.toString(16) })">
      <template #header-extra><n-tag size="small" :type="stream.state === 'failed' ? 'error' : stream.state === 'streaming' ? 'success' : 'warning'">{{ t(labels[stream.state]) }}</n-tag></template>
      <dl>
        <div><dt>USB</dt><dd>{{ stream.bus }}:{{ stream.address }}</dd></div>
        <div><dt>{{ t("service.desktopStream.backend") }}</dt><dd>{{ stream.backend ?? t("service.desktopStream.pending") }}</dd></div>
        <div><dt>{{ t("service.desktopStream.transfer") }}</dt><dd>{{ !stream.encoding || stream.encoding.encoder === 'unknown' ? '—' : stream.encoding.encoder === 'turbojpeg' ? 'JPEG' : 'H.264' }}</dd></div>
        <div><dt>{{ t("service.desktopStream.fpsLimit") }}</dt><dd>{{ stream.applied_policy?.fps_limit ?? '—' }}</dd></div>
        <div><dt>{{ t("service.desktopStream.encoder") }}</dt><dd>{{ stream.encoding?.encoder ?? '—' }}</dd></div>
        <div><dt>{{ t("service.desktopStream.input") }}</dt><dd>{{ !stream.encoding ? '—' : stream.encoding.gpu_input ? 'GPU' : 'CPU' }}</dd></div>
        <div><dt>{{ t("service.desktopStream.hardwareVideo") }}</dt><dd>{{ !stream.applied_policy ? '—' : t(stream.applied_policy.hardware_video ? 'common.enabled' : 'common.disabled') }}</dd></div>
      </dl>
      <p v-if="stream.error" class="failure">{{ backendText(stream.error, stream.error_ui) }}</p>
      <details v-if="stream.fallback_reason || stream.encoding?.cpu_readback_reason || stream.encoding?.software_reason">
        <summary>{{ t("service.desktopStream.fallbackDetails") }}</summary>
        <p v-if="stream.encoding?.cpu_readback_reason">{{ t(stream.encoding.cpu_readback_reason === 'no_dma_buf' ? 'service.desktopStream.cpuOnlyBackend' : 'service.desktopStream.gpuFailed') }}</p>
        <p v-if="stream.encoding?.software_reason">{{ t(stream.encoding.software_reason === 'hardware_failed' ? 'service.desktopStream.hardwareEncoderFailed' : 'service.desktopStream.noHardwareEncoder') }}</p>
        <p v-if="stream.fallback_reason">{{ stream.fallback_reason }}</p>
      </details>
      <n-button v-if="stream.state === 'failed' && daemon.info?.capabilities.includes('desktop_retry')" size="small"
        :disabled="!daemon.canWrite || retrying" :loading="retrying" @click="retry(stream)">{{ t("common.retry") }}</n-button>
    </n-card>
  </div>
</template>

<style scoped>
.stream-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(min(100%, 320px), 1fr)); gap: var(--space-3); }
dl { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: var(--space-3); margin: 0; }
dt, summary { color: var(--text-secondary); }
dd { margin: 2px 0 0; overflow-wrap: anywhere; }
p { margin: var(--space-2) 0; overflow-wrap: anywhere; }
details { margin-top: var(--space-3); }
summary { cursor: pointer; }
.failure { color: var(--danger); }
</style>
