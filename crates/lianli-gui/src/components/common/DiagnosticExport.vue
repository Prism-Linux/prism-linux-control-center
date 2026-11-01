<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { invoke } from "@/utils/backendText";
import { useMessage } from "naive-ui";
import { useInstallationStore } from "@/stores/installation";
import { useDaemonStore } from "@/stores/daemon";
import { useTranslate } from "@/i18n";

const t = useTranslate();
const installation = useInstallationStore();
const daemon = useDaemonStore();
const message = useMessage();
const logSource = ref<string | null>(null);
const options = computed(() => [
  { label: t("service.diagnostics.sourceAuto"), value: "auto" },
  { label: t("service.diagnostics.sourceUser"), value: "user_daemon" },
  { label: t("service.diagnostics.sourceSystem"), value: "system_daemon" },
]);
const preview = ref<{ id: number; text: string; can_save: boolean } | null>(null);
const loading = ref(false);
const saving = ref(false);
const error = ref("");
const show = ref(false);
watch(logSource, () => { preview.value = null; });

async function prepare(savedFile = false) {
  if (!installation.report || loading.value) return;
  loading.value = true;
  error.value = "";
  try {
    const result = await invoke<typeof preview.value>(savedFile ? "diagnostic_preview_file" : "diagnostic_preview", { input: {
      report: installation.report, daemon: daemon.info,
      media: daemon.mediaPreparation,
      desktop_streams: daemon.desktopStreams,
      log_source: logSource.value === "auto" ? null : logSource.value,
    }, ...(savedFile ? { title: t("service.diagnostics.pickLogsTitle") } : {}) });
    if (result) {
      preview.value = result;
      show.value = true;
    }
  } catch (reason) {
    error.value = String(reason);
  } finally {
    loading.value = false;
  }
}

async function save() {
  if (!preview.value?.can_save || saving.value) return;
  saving.value = true;
  try {
    if (await invoke<boolean>("diagnostic_save", { id: preview.value.id })) {
      message.success(t("service.diagnostics.saved"));
    }
  } catch (reason) {
    message.error(String(reason));
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <n-card :title="t('service.diagnostics.title')">
    <p>{{ t("service.diagnostics.intro") }}</p>
    <n-form-item :label="t('service.diagnostics.logSource')">
      <n-select :value="logSource ?? 'auto'" :options="options" :disabled="loading || saving" @update:value="logSource = $event" />
    </n-form-item>
    <p>{{ t("service.diagnostics.logsHint") }}</p>
    <n-alert v-if="error" type="error">{{ error }}</n-alert>
    <n-space>
      <n-button :loading="loading" :disabled="!installation.report || installation.checking || saving" @click="prepare()">{{ t("service.diagnostics.previewExport") }}</n-button>
      <n-button :disabled="loading || saving || !installation.report" @click="prepare(true)">{{ t("service.diagnostics.chooseSavedLog") }}</n-button>
    </n-space>
    <n-modal v-model:show="show" preset="card" :title="t('service.diagnostics.reviewTitle')" class="diagnostic-preview" :mask-closable="!saving" :closable="!saving">
      <p>{{ t("service.diagnostics.snapshotNote") }}</p>
      <n-alert v-if="preview && !preview.can_save" type="error">
        {{ t("service.diagnostics.noLogs") }}
      </n-alert>
      <n-button v-if="preview && !preview.can_save" type="primary" class="saved-log-button" :disabled="loading || saving" @click="prepare(true)">{{ t("service.diagnostics.chooseSavedLog") }}</n-button>
      <n-input :value="preview?.text ?? ''" type="textarea" readonly :autosize="{ minRows: 12, maxRows: 24 }" />
      <template #footer><n-button type="primary" :loading="saving" :disabled="!preview?.can_save" @click="save">{{ t("service.diagnostics.saveJson") }}</n-button></template>
    </n-modal>
  </n-card>
</template>

<style scoped>
.diagnostic-preview { width: min(900px, 92vw); }
.n-alert, .saved-log-button { margin-bottom: var(--space-3); }
p { margin: 0 0 var(--space-3); }
</style>
