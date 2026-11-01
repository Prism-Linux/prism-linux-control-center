<script setup lang="ts">
import { ref, watch } from "vue";
import { useIpc } from "@/composables/useIpc";
import { useDaemonStore } from "@/stores/daemon";
import { useTranslate, type MessageKey } from "@/i18n";
import { backendText, type UiMessage } from "@/utils/backendText";

type Target = { kind: "configuration" | "templates" | "rgb_presets" } | { kind: "profile"; name: string };
type Entry = { target: Target; preserved?: boolean; bytes: number; modified_unix_seconds: number | null; error: string | null; error_ui?: UiMessage | null };
type Preview = { target: Target; preserved?: boolean; sha256: string; bytes: number; json: string; truncated: boolean; parse_error: string | null; parse_error_ui?: UiMessage | null; validation_error?: string | null; validation_error_ui?: UiMessage | null; warnings?: string[]; warnings_ui?: (UiMessage | null)[] };
const t = useTranslate();
const daemon = useDaemonStore();
const ipc = useIpc();
const entries = ref<Entry[] | null>(null);
const preview = ref<Preview | null>(null);
const show = ref(false);
const busy = ref(false);
const error = ref("");
const confirmed = ref(false);
const deleteConfirmed = ref(false);
const resultMessage = ref<MessageKey | null>(null);
watch(preview, () => { confirmed.value = false; deleteConfirmed.value = false; });
watch(() => daemon.info?.instance_id, () => { entries.value = null; preview.value = null; show.value = false; error.value = ""; resultMessage.value = null; });
const targetLabels: Record<Exclude<Target["kind"], "profile">, MessageKey> = {
  configuration: "service.stateBackups.targetConfiguration",
  templates: "service.stateBackups.targetTemplates",
  rgb_presets: "service.stateBackups.targetRgbPresets",
};
function label(target: Target) {
  return target.kind === "profile" ? t("service.stateBackups.targetProfile", { name: target.name }) : t(targetLabels[target.kind]);
}
async function read(target?: Target, preserved = false) {
  if (busy.value) return;
  busy.value = true;
  const instance = daemon.info?.instance_id;
  error.value = "";
  resultMessage.value = null;
  try {
    if (target) {
      const result = await ipc.request<Preview>("PreviewStateBackup", { target, preserved });
      if (daemon.info?.instance_id !== instance) return;
      preview.value = result;
      show.value = true;
    } else {
      const result = await ipc.request<Entry[]>("ListStateBackups");
      if (daemon.info?.instance_id === instance) entries.value = result;
    }
  } catch (reason) { if (daemon.info?.instance_id === instance) error.value = String(reason); }
  finally { busy.value = false; }
}

async function restore() {
  const selected = preview.value;
  if (!selected || selected.preserved || !confirmed.value || busy.value || !daemon.canWrite) return;
  const instance = daemon.info?.instance_id;
  busy.value = true;
  error.value = "";
  try {
    await ipc.request("RestoreStateBackup", { target: selected.target, sha256: selected.sha256 });
    if (daemon.info?.instance_id !== instance) return;
    show.value = false;
    preview.value = null;
    entries.value = null;
    resultMessage.value = "service.stateBackups.restored";
  } catch (reason) { if (daemon.info?.instance_id === instance) error.value = String(reason); }
  finally { busy.value = false; }
}

async function remove() {
  const selected = preview.value;
  if (!selected || !deleteConfirmed.value || busy.value || !daemon.canWrite) return;
  const instance = daemon.info?.instance_id;
  busy.value = true;
  error.value = "";
  try {
    await ipc.request("DeleteStateBackup", { target: selected.target, preserved: selected.preserved ?? false, sha256: selected.sha256 });
    if (daemon.info?.instance_id !== instance) return;
    show.value = false;
    preview.value = null;
    entries.value = null;
    resultMessage.value = "service.stateBackups.deleted";
  } catch (reason) { if (daemon.info?.instance_id === instance) error.value = String(reason); }
  finally { busy.value = false; }
}
</script>

<template>
  <n-card :title="t('service.stateBackups.title')">
    <p>{{ t("service.stateBackups.intro") }}</p>
    <n-button :loading="busy" :disabled="!daemon.connected || !daemon.info?.capabilities.includes('backup_preview')" @click="read()">{{ t("service.stateBackups.findBackups") }}</n-button>
    <n-alert v-if="error" type="error">{{ error }}</n-alert>
    <n-alert v-if="resultMessage" type="info">{{ t(resultMessage) }}</n-alert>
    <p v-if="entries?.length === 0">{{ t("service.stateBackups.noBackups") }}</p>
    <n-space v-for="entry in entries" :key="JSON.stringify([entry.target, entry.preserved])" justify="space-between" align="center">
      <div>{{ label(entry.target) }} · {{ t(entry.preserved ? 'service.stateBackups.beforeRestore' : 'service.stateBackups.previousSave') }} · {{ t("service.stateBackups.bytes", { count: entry.bytes.toLocaleString() }) }}
        <span v-if="entry.modified_unix_seconds"> · {{ new Date(entry.modified_unix_seconds * 1000).toLocaleString() }}</span>
        <p v-if="entry.error">{{ backendText(entry.error, entry.error_ui) }}</p>
      </div>
      <n-button :disabled="busy || !!entry.error || !daemon.connected" @click="read(entry.target, entry.preserved)">{{ t("service.stateBackups.preview") }}</n-button>
    </n-space>
    <n-modal v-model:show="show" preset="card" :title="preview ? label(preview.target) : t('service.stateBackups.previewTitle')" style="width: min(900px, 92vw)" :content-style="{ maxHeight: '65vh', overflowY: 'auto' }">
      <p>{{ t("service.stateBackups.privacyNote") }}</p>
      <p>{{ t(preview?.preserved ? 'service.stateBackups.preservedState' : 'service.stateBackups.previousState') }}</p>
      <n-alert v-if="error" type="error">{{ error }}</n-alert>
      <n-alert v-if="preview?.parse_error" type="error">{{ t("service.stateBackups.invalidJson", { error: backendText(preview.parse_error, preview.parse_error_ui) }) }}</n-alert>
      <n-alert v-else-if="preview?.validation_error" type="error">{{ t("service.stateBackups.validationFailed", { error: backendText(preview.validation_error, preview.validation_error_ui) }) }}</n-alert>
      <n-alert v-for="(warning, index) in preview?.warnings ?? []" :key="index" type="warning">{{ backendText(warning, preview?.warnings_ui?.[index]) }}</n-alert>
      <n-alert v-if="preview?.truncated" type="warning">{{ t("service.stateBackups.truncated") }}</n-alert>
      <n-input :value="preview?.json ?? ''" type="textarea" readonly :autosize="{ minRows: 10, maxRows: 22 }" />
      <template v-if="!preview?.preserved">
        <p>{{ t("service.stateBackups.restoreWarning") }}</p>
        <n-checkbox v-model:checked="confirmed">{{ t("service.stateBackups.restoreConfirm") }}</n-checkbox>
      </template>
      <p>{{ t("service.stateBackups.deleteWarning") }}</p>
      <n-checkbox v-model:checked="deleteConfirmed">{{ t("service.stateBackups.deleteConfirm") }}</n-checkbox>
      <template #footer><n-space justify="end">
        <n-button @click="show = false">{{ t("common.close") }}</n-button>
        <n-button type="error" :loading="busy" :disabled="!deleteConfirmed || busy || !daemon.canWrite || !daemon.info?.capabilities.includes('backup_cleanup')" @click="remove">{{ t("service.stateBackups.deleteBackup") }}</n-button>
        <n-button v-if="!preview?.preserved" type="warning" :loading="busy" :disabled="!confirmed || busy || !daemon.canWrite || !daemon.info?.capabilities.includes('backup_restore') || !!preview?.parse_error || !!preview?.validation_error" @click="restore">{{ t("service.stateBackups.restoreBackup") }}</n-button>
      </n-space></template>
    </n-modal>
  </n-card>
</template>
