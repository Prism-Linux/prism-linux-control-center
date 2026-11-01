import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { invoke } from "@/utils/backendText";
import type { InstallationReport } from "@/types/installation";
import { useDaemonStore } from "@/stores/daemon";
import { tr } from "@/i18n";

export const useInstallationStore = defineStore("installation", () => {
  const report = ref<InstallationReport | null>(null);
  const checking = ref(false);
  const error = ref("");
  const popupOpen = ref(false);
  const issues = computed(() => report.value?.findings.filter(
    (finding) => finding.severity !== "info" &&
      (finding.state === "failed" || finding.state === "unavailable"),
  ) ?? []);
  const contextLabel = computed(() => {
    const label = (context?: InstallationReport["context"] | null) => {
      if (!context) return tr("service.installation.contextUnverified");
      if (context.kind === "distrobox") return tr("service.installation.contextDistrobox", { name: context.name });
      return tr(context.kind === "native" ? "service.installation.contextNative" : "service.installation.contextUnsupported");
    };
    return tr("service.installation.contextLine", { gui: label(report.value?.context), daemon: label(report.value?.daemon_context) });
  });
  let pending: Promise<void> | null = null;
  let started = false;

  function recheck(): Promise<void> {
    if (pending) return pending;
    checking.value = true;
    error.value = "";
    pending = invoke<InstallationReport>("installation_health")
      .then((result) => { report.value = result; })
      .catch((reason: unknown) => { error.value = String(reason); })
      .finally(() => {
        checking.value = false;
        pending = null;
      });
    return pending;
  }

  async function start() {
    if (started) return;
    started = true;
    const daemon = useDaemonStore();
    await Promise.all([recheck(), daemon.refresh()]);
    popupOpen.value = issues.value.length > 0 || !!error.value || !daemon.connected;
  }

  return { report, checking, error, issues, contextLabel, popupOpen, recheck, start };
});
