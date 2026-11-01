<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { backendText, invoke } from "@/utils/backendText";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useDialog } from "naive-ui";
import { useConfigStore } from "@/stores/config";
import type { ContainerDeployment, ServiceAction, ServiceChangeRequest, ServiceOperationStatus, ServiceScope } from "@/types/installation";
import { useInstallationStore } from "@/stores/installation";
import { useDaemonStore } from "@/stores/daemon";
import { canSetUpServices, canSwitchServices, DAEMON_MODE_LABELS } from "@/utils/serviceSetup";
import { findingRemediation } from "@/utils/findingText";
import { useTranslate, type MessageKey } from "@/i18n";

defineProps<{ setupOnly?: boolean }>();

const t = useTranslate();
const actionLabels: Record<ServiceAction, MessageKey> = {
  start: "service.serviceStatus.action.start",
  stop: "service.serviceStatus.action.stop",
  restart: "service.serviceStatus.action.restart",
};
const actionTitles: Record<ServiceAction, MessageKey> = {
  start: "service.serviceStatus.actionTitle.start",
  stop: "service.serviceStatus.actionTitle.stop",
  restart: "service.serviceStatus.actionTitle.restart",
};
const actionConfirmations: Record<ServiceAction, MessageKey> = {
  start: "service.serviceStatus.actionConfirm.start",
  stop: "service.serviceStatus.actionConfirm.stop",
  restart: "service.serviceStatus.actionConfirm.restart",
};
const setupIntroKey: MessageKey = "service.serviceStatus.setupIntro";
const installation = useInstallationStore();
const daemon = useDaemonStore();
const config = useConfigStore();
const dialog = useDialog();
const operation = ref<ServiceOperationStatus>({ active: false, message: "", success: null });
const submitting = ref(false);
const submissionError = ref("");
const setupOpen = ref(false);
const setupProposal = ref<ContainerDeployment | null>(null);
const lastUserConfig = ref<string | null>(null);
watch(() => daemon.info, (info) => {
  if (info?.mode === "user") lastUserConfig.value = info.config_path;
}, { immediate: true });
const destination = ref<ServiceScope | null>(null);
const settingsChoice = ref(daemon.connected ? "carry" : "destination");
const modeOptions = computed(() => [
  { label: t("service.serviceStatus.modeUser"), value: "user" },
  { label: t("service.serviceStatus.modeSystem"), value: "system" },
]);
const settingsOptions = computed(() => [
  { label: t("service.serviceStatus.carrySettings"), value: "carry" },
  { label: t("service.serviceStatus.destinationSettings"), value: "destination" },
]);
const busy = computed(() => submitting.value || operation.value.active);
const canSwitch = computed(() => canSwitchServices(report.value));
let unlisten: UnlistenFn | undefined;
let disposed = false;
let progressEvents = 0;
let timer: ReturnType<typeof setTimeout> | undefined;
let refreshing: Promise<boolean> | undefined;

async function completed() {
  await daemon.refresh();
  if (daemon.connected && !config.dirty) await config.load();
  await installation.recheck();
}

async function prepareSetup() {
  submitting.value = true;
  submissionError.value = "";
  try {
    setupProposal.value = await invoke<ContainerDeployment>("container_setup_proposal", { userConfig: lastUserConfig.value });
    setupOpen.value = true;
  } catch (error) { submissionError.value = String(error); }
  finally { submitting.value = false; }
}

async function installSupport() {
  if (!setupProposal.value) return;
  submitting.value = true;
  submissionError.value = "";
  const before = progressEvents;
  try {
    await invoke<void>("container_setup", { deployment: setupProposal.value });
    setupOpen.value = false;
    if (before === progressEvents) await completed();
  } catch (error) { submissionError.value = String(error); }
  finally { submitting.value = false; }
}

async function refreshOperation() {
  if (refreshing) return refreshing;
  refreshing = (async () => {
    const previous = progressEvents;
    const status = await invoke<ServiceOperationStatus>("service_operation_status");
    if (!disposed && progressEvents === previous) {
      const finished = operation.value.active && !status.active;
      operation.value = status;
      if (finished) await completed();
      return finished;
    }
    return false;
  })();
  try { return await refreshing; }
  finally { refreshing = undefined; }
}

function scheduleProgress() {
  if (disposed) return;
  timer = setTimeout(async () => {
    try { await refreshOperation(); }
    catch (error) { submissionError.value = String(error); }
    finally { scheduleProgress(); }
  }, operation.value.active ? 1000 : 5000);
}

onMounted(async () => {
  if (!installation.report) void installation.recheck();
  try {
    const stop = await listen<ServiceOperationStatus>("service-operation", (event) => {
      progressEvents++;
      const finished = operation.value.active && !event.payload.active;
      operation.value = event.payload;
      if (finished) {
        void completed().catch((error: unknown) => { submissionError.value = String(error); });
      }
    });
    if (disposed) { stop(); return; }
    unlisten = stop;
    await refreshOperation();
  } catch (error) {
    operation.value = { active: false, success: false, message: String(error) };
  } finally { scheduleProgress(); }
});
onUnmounted(() => { disposed = true; unlisten?.(); clearTimeout(timer); });

async function perform(command: "service_action" | "service_change", request: { scope: ServiceScope; action: ServiceAction } | ServiceChangeRequest) {
  submitting.value = true;
  submissionError.value = "";
  try {
    await invoke<void>(command, { request });
    const refreshed = await refreshOperation();
    if (!operation.value.active && !refreshed) await completed();
  } catch (error) {
    submissionError.value = String(error);
  } finally {
    submitting.value = false;
  }
}

function act(scope: ServiceScope, action: ServiceAction) {
  const scopeLabel = t(DAEMON_MODE_LABELS[scope]);
  dialog.warning({
    title: t(actionTitles[action], { scope: scopeLabel }),
    content: config.dirty
      ? t("service.serviceStatus.actionSaveFirst")
      : t(actionConfirmations[action], { scope: scopeLabel }),
    positiveText: t(config.dirty ? "service.serviceStatus.saveAndContinue" : actionLabels[action]),
    negativeText: t("common.cancel"),
    onPositiveClick: async () => {
      if (config.dirty) {
        try { await config.save(); }
        catch (error) {
          operation.value = { active: false, success: false, message: String(error) };
          return false;
        }
      }
      await perform("service_action", { scope, action });
    },
  });
}
function change(request: ServiceChangeRequest) {
  if (busy.value || (request.kind === "switch" && request.scope === currentMode.value)) return;
  const recovery = request.kind === "recover";
  const setup = !recovery && initialSetup.value;
  const scope = request.kind === "switch" ? t(DAEMON_MODE_LABELS[request.scope]) : "";
  dialog.warning({
    title: recovery ? t("service.serviceStatus.recoverTitle") : t(setup ? "service.serviceStatus.enableTitle" : "service.serviceStatus.switchTitle", { scope }),
    content: config.dirty
      ? t("service.serviceStatus.changeSaveFirst")
      : recovery
        ? t("service.serviceStatus.recoverConfirm")
        : setup
          ? t("service.serviceStatus.enableConfirm", { scope })
        : t(request.carry_settings ? "service.serviceStatus.switchConfirmCarry" : "service.serviceStatus.switchConfirmDestination", { scope }),
    positiveText: t(config.dirty ? "common.close" : recovery ? "service.serviceStatus.recover" : setup ? "service.serviceStatus.enableAndStart" : "service.serviceStatus.switchService"),
    negativeText: t("common.cancel"),
    onPositiveClick: async () => {
      if (!config.dirty) await perform("service_change", request);
    },
  });
}
const report = computed(() => installation.report?.services);
const lingeringIssue = computed(() => installation.report?.findings.find(
  (finding) => finding.code === "services.distrobox_lingering" && finding.state !== "passed",
));
const initialSetup = computed(() => !daemon.connected && canSetUpServices(report.value));
const currentMode = computed<ServiceScope | null>(() => {
  if (daemon.connected && daemon.info?.mode && daemon.info.mode !== "unknown") return daemon.info.mode;
  const selection = report.value?.selection;
  if (selection?.state === "known" && selection.value) return selection.value.scope;
  if (report.value?.user.state === "known" && report.value.user.value.active_state === "active") return "user";
  if (report.value?.system.state === "known" && report.value.system.value.active_state === "active") return "system";
  return null;
});
watch(currentMode, (mode) => { if (mode) destination.value = mode; }, { immediate: true });
const controlsReady = computed(() => report.value?.operation_lock?.state === "known");
const units = computed(() => report.value ? [
  { label: t("service.serviceStatus.hostUserService"), scope: "user" as const, probe: report.value.user },
  { label: t("service.serviceStatus.hostSystemService"), scope: "system" as const, probe: report.value.system },
] : []);
const owner = computed(() => {
  const ownership = report.value?.ownership;
  if (ownership?.state !== "known") return t("service.serviceStatus.unverified");
  if (ownership.value.owner_pid === null) return t("common.none");
  const process = ownership.value.process;
  if (!process) return t("service.serviceStatus.processUnchecked");
  if (process.state === "unavailable") return t("service.serviceStatus.ownerUnverified", { reason: backendText(process.reason, process.reason_ui) });
  const label: MessageKey = process.value.service === "user" ? "service.serviceStatus.ownerUserService"
    : process.value.service === "system" ? "service.serviceStatus.ownerSystemService"
    : "service.serviceStatus.ownerOther";
  return t(label, { uid: process.value.effective_uid });
});
const lockMatch = computed(() => {
  const held = daemon.info?.ownership_lock;
  const inspected = report.value?.ownership;
  if (!held || inspected?.state !== "known") return null;
  return held.device === inspected.value.identity.device && held.inode === inspected.value.identity.inode;
});
</script>

<template>
  <n-card :title="t(initialSetup ? 'service.serviceStatus.titleSetup' : 'service.serviceStatus.title')" size="small">
    <template #header-extra>
      <n-button size="small" :loading="installation.checking" :disabled="busy" @click="installation.recheck">{{ t("service.serviceStatus.recheck") }}</n-button>
    </template>
    <n-alert v-if="submissionError || installation.error" type="error">{{ submissionError || installation.error }}</n-alert>
    <n-alert v-if="operation.message" :type="operation.success === false ? 'error' : operation.success === true ? 'success' : 'info'">{{ backendText(operation.message, operation.message_ui) }}</n-alert>
    <p v-if="initialSetup">{{ t("service.serviceStatus.setupIntroLine") }}</p>
    <p v-if="!report" class="muted">{{ t(installation.checking ? "service.serviceStatus.checking" : "service.serviceStatus.statusUnavailable") }}</p>
    <p v-if="report?.operation_lock?.state === 'unavailable'">{{ t("service.serviceStatus.controlsUnavailable", { reason: backendText(report.operation_lock.reason, report.operation_lock.reason_ui) }) }}</p>
    <div v-if="(canSwitch && controlsReady) || report?.context.kind === 'distrobox'" class="switch-row">
      <template v-if="canSwitch && controlsReady">
        <n-select v-model:value="destination" :options="modeOptions" :placeholder="t('service.serviceStatus.selectMode')" :disabled="busy || installation.checking" class="mode-select" />
        <n-select v-if="!initialSetup && destination && destination !== currentMode" v-model:value="settingsChoice"
          :options="settingsOptions"
          :disabled="busy || installation.checking" class="settings-select" />
        <n-button :disabled="busy || installation.checking || !destination || destination === currentMode"
          @click="destination && change({ kind: 'switch', scope: destination, carry_settings: !initialSetup && settingsChoice === 'carry' })">{{ t(initialSetup ? 'service.serviceStatus.enableAndStart' : 'service.serviceStatus.switchService') }}</n-button>
      </template>
      <n-button v-if="report?.context.kind === 'distrobox'" :disabled="busy || installation.checking || daemon.connected || config.dirty" @click="prepareSetup">{{ t(canSwitch ? 'service.serviceStatus.repairHostSupport' : 'service.serviceStatus.setUpHostSupport') }}</n-button>
    </div>
    <p v-if="!(canSwitch && controlsReady) && report?.context.kind === 'distrobox'" class="muted">{{ t("service.serviceStatus.setUpWrappers") }}</p>
    <n-alert v-if="destination === 'system' && lingeringIssue" type="warning" :title="t('service.serviceStatus.manualSetupRequired')">{{ findingRemediation(lingeringIssue) }}</n-alert>
    <n-space v-if="report?.context.kind === 'distrobox'" vertical>
      <span v-if="daemon.connected" class="muted">{{ t("service.serviceStatus.stopDaemonFirst") }}</span>
      <span v-else-if="config.dirty" class="muted">{{ t("service.serviceStatus.saveEditsFirst") }}</span>
    </n-space>
    <div v-if="!setupOnly" class="units">
      <div v-for="unit in units" :key="unit.scope" class="unit">
        <div class="unit-heading"><strong>{{ unit.label }}</strong>
          <n-tag v-if="unit.probe.state === 'known'" size="small" :type="unit.probe.value.active_state === 'active' ? 'success' : 'default'">{{ unit.probe.value.active_state }}</n-tag>
        </div>
        <template v-if="unit.probe.state === 'known'">
          <p class="muted">{{ t("service.serviceStatus.startup", { state: unit.probe.value.unit_file_state }) }}<span v-if="unit.probe.value.distrobox_name"> · {{ t("service.serviceStatus.daemonDistrobox", { name: unit.probe.value.distrobox_name }) }}</span></p>
          <n-space v-if="controlsReady && unit.probe.value.load_state === 'loaded' && (unit.scope === 'user' || canSwitch)">
            <n-button size="small" :disabled="busy || installation.checking || !['inactive', 'failed'].includes(unit.probe.value.active_state)" @click="act(unit.scope, 'start')">{{ t(actionLabels.start) }}</n-button>
            <n-button size="small" :disabled="busy || installation.checking || unit.probe.value.active_state !== 'active'" @click="act(unit.scope, 'stop')">{{ t(actionLabels.stop) }}</n-button>
            <n-button size="small" :disabled="busy || installation.checking || unit.probe.value.active_state !== 'active'" @click="act(unit.scope, 'restart')">{{ t(actionLabels.restart) }}</n-button>
          </n-space>
        </template>
        <p v-else>{{ backendText(unit.probe.reason, unit.probe.reason_ui) }}</p>
      </div>
    </div>
    <n-alert v-if="daemon.connected && lockMatch === false" type="error">{{ t("service.serviceStatus.lockMismatchAlert") }}</n-alert>
    <details v-if="report && !setupOnly">
      <summary>{{ t("service.serviceStatus.ownershipRecovery") }}</summary>
      <dl v-if="daemon.connected && daemon.info" class="daemon-details">
        <div><dt>{{ t("service.serviceStatus.daemon") }}</dt><dd>{{ daemon.info.version }} · {{ t(DAEMON_MODE_LABELS[daemon.info.mode]) }} · PID {{ daemon.info.pid }}</dd></div>
        <div><dt>{{ t("service.serviceStatus.instance") }}</dt><dd>{{ daemon.info.instance_id }}</dd></div>
        <div class="config-path"><dt>{{ t("service.serviceStatus.configuration") }}</dt><dd>{{ daemon.info.config_path }}</dd></div>
      </dl>
      <p>{{ t("service.serviceStatus.owner", { owner }) }}</p>
      <p v-if="report.selection?.state === 'known'">{{ t("service.serviceStatus.startupSelection", { selection: report.selection.value ? t(DAEMON_MODE_LABELS[report.selection.value.scope]) : t("service.serviceStatus.notConfigured") }) }}</p>
      <p v-else-if="report.selection?.state === 'unavailable'">{{ backendText(report.selection.reason, report.selection.reason_ui) }}</p>
      <p v-if="report.global_user.state === 'known'">{{ t("service.serviceStatus.globalUserStartup", { state: report.global_user.value }) }}</p>
      <p v-else>{{ backendText(report.global_user.reason, report.global_user.reason_ui) }}</p>
      <p v-if="report.ownership?.state === 'unavailable'">{{ backendText(report.ownership.reason, report.ownership.reason_ui) }}</p>
      <p v-if="daemon.connected">{{ t("service.serviceStatus.ownershipLock", { state: t(lockMatch === true ? 'service.serviceStatus.lockMatched' : lockMatch === false ? 'service.serviceStatus.lockMismatch' : 'service.serviceStatus.unverified') }) }}</p>
      <div v-for="unit in units" :key="unit.scope">
        <p v-if="unit.probe.state === 'known'">{{ t("service.serviceStatus.unitDetails", { label: unit.label, load: unit.probe.value.load_state, sub: unit.probe.value.sub_state, pid: unit.probe.value.main_pid || '—' }) }}<br />{{ unit.probe.value.fragment_path }}</p>
      </div>
      <n-button v-if="canSwitch && controlsReady" size="small" :disabled="busy || installation.checking" @click="change({ kind: 'recover' })">{{ t("service.serviceStatus.recoverInterrupted") }}</n-button>
    </details>
  </n-card>
  <n-modal v-model:show="setupOpen" preset="card" :title="t('service.serviceStatus.setupDialogTitle')" style="width: min(620px, 92vw)" :closable="!busy" :mask-closable="!busy">
    <template v-if="setupProposal">
      <i18n-t :keypath="setupIntroKey" tag="p" scope="global"><template #name><strong>{{ setupProposal.route.launch.name }}</strong></template></i18n-t>
      <n-alert v-if="submissionError" type="error">{{ submissionError }}</n-alert>
      <n-form label-placement="top" size="small">
        <n-form-item :label="t('service.serviceStatus.userConfiguration')"><n-input v-model:value="setupProposal.route.user_config" :disabled="busy" /></n-form-item>
        <n-form-item :label="t('service.serviceStatus.systemConfiguration')"><n-input v-model:value="setupProposal.route.system_config" :disabled="busy" /></n-form-item>
        <n-collapse><n-collapse-item :title="t('service.serviceStatus.launchPaths')" name="paths">
          <n-form-item :label="t('service.serviceStatus.boxBinaries')"><n-input v-model:value="setupProposal.route.launch.binaries" :disabled="busy" /></n-form-item>
          <n-form-item :label="t('service.serviceStatus.hostEnter')"><n-input v-model:value="setupProposal.route.launch.host_enter" :disabled="busy" /></n-form-item>
          <n-form-item :label="t('service.serviceStatus.userWorkingDirectory')"><n-input v-model:value="setupProposal.route.user_working_directory" :disabled="busy" /></n-form-item>
          <n-form-item :label="t('service.serviceStatus.systemWorkingDirectory')"><n-input v-model:value="setupProposal.route.system_working_directory" :disabled="busy" /></n-form-item>
        </n-collapse-item></n-collapse>
      </n-form>
      <n-space justify="end">
        <n-button :disabled="busy" @click="setupOpen = false">{{ t("common.cancel") }}</n-button>
        <n-button type="primary" :loading="submitting" :disabled="busy || daemon.connected || config.dirty" @click="installSupport">{{ t("service.serviceStatus.installHostSupport") }}</n-button>
      </n-space>
    </template>
  </n-modal>
</template>

<style scoped>
.daemon-details { display: grid; grid-template-columns: repeat(auto-fit, minmax(min(100%, 240px), 1fr)); gap: var(--space-3); }
.daemon-details dt { color: var(--text-secondary); }
.daemon-details dd { margin: 2px 0 0; overflow-wrap: anywhere; }
.config-path { grid-column: 1 / -1; }
.n-alert { margin-bottom: var(--space-3); }
.switch-row, .unit-heading { display: flex; flex-wrap: wrap; align-items: center; gap: var(--space-3); }
.mode-select { width: 180px; }
.settings-select { width: 240px; }
.units { display: grid; grid-template-columns: repeat(auto-fit, minmax(230px, 1fr)); gap: var(--space-3); margin-top: var(--space-3); }
.unit { padding: var(--space-3); border: 1px solid var(--border); border-radius: var(--radius-md); }
.unit-heading { justify-content: space-between; }
p { margin: var(--space-2) 0; overflow-wrap: anywhere; }
details { margin-top: var(--space-3); }
summary { cursor: pointer; color: var(--text-secondary); }
</style>
