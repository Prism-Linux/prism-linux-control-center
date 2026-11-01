import { tr } from "@/i18n";

export async function boundedFetch(url: string, limit: number, parent: AbortSignal, timeoutMs = 15000): Promise<Blob> {
  const controller = new AbortController();
  const abort = () => controller.abort(parent.reason);
  parent.addEventListener("abort", abort, { once: true });
  if (parent.aborted) abort();
  const timeout = setTimeout(() => controller.abort(new Error(tr("shell.download.timedOut"))), timeoutMs);
  try {
    controller.signal.throwIfAborted();
    const response = await fetch(url, { signal: controller.signal });
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    if (Number(response.headers.get("content-length")) > limit) throw new Error(tr("shell.download.tooLarge", { limit }));
    if (!response.body) throw new Error(tr("shell.download.noBody"));
    const reader = response.body.getReader();
    const chunks: BlobPart[] = [];
    let size = 0;
    try {
      while (true) {
        controller.signal.throwIfAborted();
        const { done, value } = await reader.read();
        if (done) break;
        size += value.byteLength;
        if (size > limit) throw new Error(tr("shell.download.tooLarge", { limit }));
        chunks.push(new Uint8Array(value));
      }
    } finally {
      void reader.cancel().catch(() => undefined);
      reader.releaseLock();
    }
    return new Blob(chunks, { type: response.headers.get("content-type") ?? "application/octet-stream" });
  } finally {
    clearTimeout(timeout);
    parent.removeEventListener("abort", abort);
    controller.abort();
  }
}
