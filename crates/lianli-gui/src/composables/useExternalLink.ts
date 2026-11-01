import { invoke } from "@/utils/backendText";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { useMessage } from "naive-ui";
import { tr } from "@/i18n";

export function useExternalLink() {
  const message = useMessage();

  return async function openExternal(url: string) {
    try {
      await invoke("open_external", { url });
    } catch (error) {
      const copied = await writeText(url).then(() => true, () => false);
      message.error(
        copied
          ? tr("shell.externalLink.openFailedCopied", { error: String(error) })
          : tr("shell.externalLink.openFailed", { error: String(error), url }),
        { duration: 8000, closable: true },
      );
    }
  };
}
