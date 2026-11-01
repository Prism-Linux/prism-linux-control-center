<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from "vue";
import { useIpc } from "@/composables/useIpc";
import { useDaemonStore } from "@/stores/daemon";
import { useTranslate, type MessageKey } from "@/i18n";
import { backendText, type UiMessage } from "@/utils/backendText";

interface Entry {
  directory: string;
  bytes: number;
  ownership_verified: boolean;
  issue: string | null;
  issue_ui?: UiMessage | null;
  saved_references?: string[];
  saved_references_checked?: boolean;
  saved_reference_count?: number;
  runtime_referenced?: boolean;
  runtime_references_checked?: boolean;
}

interface Review {
  references: Entry;
  contents: {
    directory: string; sha256: string; bytes: number;
    files: { path: string; bytes: number; sha256: string; matches_catalog: boolean | null }[];
    missing_files: string[];
  };
}
interface ReviewStatus { operation_id: string; finished: boolean; review: Review | null; error: string | null; error_ui?: UiMessage | null; removed?: boolean }

const props = defineProps<{ managed?: boolean }>();
const t = useTranslate();
const daemon = useDaemonStore();
const ipc = useIpc();
const entries = ref<Entry[] | null>(null);
const busy = ref(false);
const error = ref("");
const query = ref("");
const page = ref(1);
const review = ref<Review | null>(null);
const reviewPage = ref(1);
const missingPage = ref(1);
const missingFilesPage = computed(() => review.value?.contents.missing_files.slice((missingPage.value - 1) * 32, missingPage.value * 32) ?? []);
const reviewFilesPage = computed(() => review.value?.contents.files.slice((reviewPage.value - 1) * 32, reviewPage.value * 32) ?? []);
const reviewError = ref("");
const showReview = ref(false);
const reviewedOperation = ref("");
const removalOperation = ref("");
const confirmed = ref(false);
const removalNotice = ref<MessageKey | null>(null);
const canRemove = computed(() => daemon.info?.capabilities.includes(props.managed ? "managed_media_removal" : "catalog_cleanup_removal")
  && review.value?.references.saved_references_checked === true
  && review.value?.references.runtime_references_checked === true
  && review.value?.references.saved_reference_count === 0
  && review.value?.references.saved_references?.length === 0
  && review.value?.references.runtime_referenced === false && !!reviewedOperation.value);
let requestId = 0;
const supported = computed(() => daemon.connected && daemon.info?.capabilities.includes(props.managed ? "managed_media_storage" : "catalog_storage"));
const totalBytes = computed(() => entries.value?.reduce((sum, entry) => sum + entry.bytes, 0) ?? 0);
const filtered = computed(() => {
  const search = query.value.trim().toLowerCase();
  return entries.value?.filter((entry) => !search || entry.directory.toLowerCase().includes(search)
    || entry.saved_references?.some((source) => source.toLowerCase().includes(search))) ?? [];
});
const visible = computed(() => filtered.value.slice((page.value - 1) * 20, page.value * 20));
watch(query, () => { page.value = 1; });
watch([() => daemon.connected, () => daemon.info?.instance_id], () => {
  requestId++;
  entries.value = null;
  error.value = "";
  busy.value = false;
  page.value = 1;
  review.value = null;
  reviewError.value = "";
  showReview.value = false;
  reviewedOperation.value = "";
  removalOperation.value = "";
  removalNotice.value = null;
  confirmed.value = false;
});
onUnmounted(() => { requestId++; });

function size(bytes: number) {
  return bytes >= 1024 ** 3 ? `${(bytes / 1024 ** 3).toFixed(2)} GiB`
    : bytes >= 1024 ** 2 ? `${(bytes / 1024 ** 2).toFixed(1)} MiB`
      : bytes >= 1024 ? `${(bytes / 1024).toFixed(1)} KiB` : t("service.catalogStorage.bytes", { count: bytes });
}

function status(entry: Entry) {
  if (!entry.ownership_verified) return t("service.catalogStorage.statusOwnershipUnverified");
  if (!entry.saved_references_checked || !entry.runtime_references_checked) return t("service.catalogStorage.statusChecksIncomplete");
  if ((entry.saved_reference_count ?? 0) > 0 || (entry.saved_references?.length ?? 0) > 0 || entry.runtime_referenced) return t("service.catalogStorage.statusProtected");
  return t("service.catalogStorage.statusReviewRequired");
}

function fileStateLabel(matches: boolean | null): MessageKey {
  if (matches === null) return "service.catalogStorage.ownershipReceipt";
  if (!matches) return "service.catalogStorage.changedOrPartial";
  return props.managed ? "service.catalogStorage.matchesImported" : "service.catalogStorage.matchesCatalog";
}

async function inspect() {
  if (busy.value || !supported.value) return;
  const id = ++requestId;
  busy.value = true;
  entries.value = null;
  error.value = "";
  page.value = 1;
  try {
    const result = await ipc.request<Entry[]>(props.managed ? "GetManagedMediaStorage" : "GetCatalogStorage");
    if (id !== requestId) return;
    if (!Array.isArray(result) || result.length > 1024 || result.some((entry) =>
      !entry || typeof entry.directory !== "string" || entry.directory.length > 512
      || !Number.isSafeInteger(entry.bytes) || entry.bytes < 0
      || typeof entry.ownership_verified !== "boolean"
      || (entry.issue !== null && (typeof entry.issue !== "string" || entry.issue.length > 2048))
      || [entry.saved_references_checked, entry.runtime_references_checked, entry.runtime_referenced].some((flag) => flag !== undefined && typeof flag !== "boolean")
      || (entry.saved_reference_count !== undefined && (!Number.isSafeInteger(entry.saved_reference_count) || entry.saved_reference_count < (entry.saved_references?.length ?? 0)))
      || (entry.saved_references !== undefined && (!Array.isArray(entry.saved_references)
        || entry.saved_references.length > 16 || entry.saved_references.some((source) => typeof source !== "string" || source.length > 512))))
      || !Number.isSafeInteger(result.reduce((sum, entry) => sum + entry.bytes, 0))) {
      throw new Error(t("service.catalogStorage.invalidReport"));
    }
    entries.value = result;
  } catch (reason) {
    if (id === requestId) error.value = String(reason);
  } finally {
    if (id === requestId) busy.value = false;
  }
}

async function reviewFiles(entry: Entry) {
  if (busy.value || !supported.value) return;
  const id = ++requestId;
  busy.value = true;
  review.value = null;
  reviewError.value = "";
  reviewedOperation.value = "";
  confirmed.value = false;
  showReview.value = true;
  try {
    reviewPage.value = 1;
    missingPage.value = 1;
    let result = await ipc.request<ReviewStatus>(props.managed ? "StartManagedMediaReview" : "StartCatalogReview", { directory: entry.directory });
    const operation = result.operation_id;
    if (typeof operation !== "string" || operation.length > 160 || typeof result.finished !== "boolean") throw new Error(t("service.catalogStorage.invalidReviewStatus"));
    while (id === requestId && !result.finished) {
      await new Promise((resolve) => setTimeout(resolve, 1000));
      if (id !== requestId) return;
      result = await ipc.request<ReviewStatus>(props.managed ? "GetManagedMediaReview" : "GetCatalogReview", { operation_id: operation });
      if (result.operation_id !== operation || typeof result.finished !== "boolean") throw new Error(t("service.catalogStorage.reviewChanged"));
    }
    if (id !== requestId) return;
    if (result.error) throw new Error(backendText(result.error, result.error_ui));
    const snapshot = result.review;
    if (!snapshot?.contents || snapshot.references?.directory !== entry.directory || snapshot.references.ownership_verified !== true
      || snapshot.references.saved_references_checked !== true || snapshot.references.runtime_references_checked !== true
      || typeof snapshot.references.runtime_referenced !== "boolean"
      || !Number.isSafeInteger(snapshot.references.saved_reference_count) || (snapshot.references.saved_reference_count ?? -1) < 0
      || !Array.isArray(snapshot.references.saved_references) || snapshot.references.saved_references.length > 16
      || snapshot.references.saved_references.some((source) => typeof source !== "string" || source.length > 512)
      || snapshot.contents.directory !== entry.directory || !/^[a-f0-9]{64}$/.test(snapshot.contents.sha256)
      || !Number.isSafeInteger(snapshot.contents.bytes) || snapshot.contents.bytes < 0 || snapshot.contents.bytes > (props.managed ? 8 * 1024 ** 3 : 256 * 1024 ** 2)
      || !Array.isArray(snapshot.contents.files) || snapshot.contents.files.length > (props.managed ? 8192 : 130)
      || snapshot.contents.files.some((file) => typeof file.path !== "string" || file.path.length > 512 || !Number.isSafeInteger(file.bytes) || file.bytes < 0
        || !/^[a-f0-9]{64}$/.test(file.sha256) || (file.matches_catalog !== null && typeof file.matches_catalog !== "boolean"))
      || !Array.isArray(snapshot.contents.missing_files) || snapshot.contents.missing_files.length > (props.managed ? 8192 : 129)
      || snapshot.contents.missing_files.some((path) => typeof path !== "string" || path.length > 512)) {
      throw new Error(t("service.catalogStorage.invalidReview"));
    }
    review.value = snapshot;
    reviewedOperation.value = operation;
  } catch (reason) {
    if (id === requestId) reviewError.value = t("service.catalogStorage.reviewFailed", { reason: String(reason) });
  } finally {
    if (id === requestId) busy.value = false;
  }
}

async function removeFiles(start: boolean) {
  if (busy.value || !daemon.connected || (start && (!canRemove.value || !confirmed.value))) return;
  const operation = start ? reviewedOperation.value : removalOperation.value;
  if (!operation) return;
  const id = ++requestId;
  removalOperation.value = operation;
  reviewedOperation.value = "";
  review.value = null;
  confirmed.value = false;
  entries.value = null;
  reviewError.value = "";
  removalNotice.value = "service.catalogStorage.removalMayRun";
  busy.value = true;
  showReview.value = true;
  try {
    const statusRequest = props.managed ? "GetManagedMediaReview" : "GetCatalogReview";
    let result = await ipc.request<ReviewStatus>(start ? props.managed ? "StartManagedMediaRemoval" : "StartCatalogRemoval" : statusRequest, { operation_id: operation });
    while (id === requestId) {
      if (result.operation_id !== operation || typeof result.finished !== "boolean"
        || (result.removed !== undefined && typeof result.removed !== "boolean")) throw new Error(t("service.catalogStorage.invalidRemovalStatus"));
      if (result.finished) break;
      await new Promise((resolve) => setTimeout(resolve, 1000));
      if (id !== requestId) return;
      result = await ipc.request<ReviewStatus>(statusRequest, { operation_id: operation });
    }
    if (id !== requestId) return;
    removalOperation.value = "";
    if (result.error) throw new Error(backendText(result.error, result.error_ui));
    if (result.removed !== true) throw new Error(t("service.catalogStorage.removalIncomplete"));
    removalNotice.value = "service.catalogStorage.removed";
  } catch (reason) {
    if (id === requestId) {
      reviewError.value = String(reason);
      removalNotice.value = removalOperation.value
        ? "service.catalogStorage.removalUnknown"
        : "service.catalogStorage.removalPartial";
    }
  } finally {
    if (id === requestId) busy.value = false;
  }
}

function closeReview() {
  requestId++;
  busy.value = false;
  showReview.value = false;
  review.value = null;
  reviewedOperation.value = "";
  confirmed.value = false;
}
</script>

<template>
  <n-card :title="t(managed ? 'service.catalogStorage.titleManaged' : 'service.catalogStorage.titleCatalog')">
    <p v-if="managed">{{ t("service.catalogStorage.introManaged") }}</p>
    <p v-else>{{ t("service.catalogStorage.introCatalog") }}</p>
    <n-button :loading="busy" :disabled="!supported || busy" @click="inspect">{{ t("service.catalogStorage.inspect") }}</n-button>
    <n-button v-if="removalOperation" :disabled="busy || !daemon.connected" @click="removeFiles(false)">{{ t("service.catalogStorage.checkRemoval") }}</n-button>
    <p v-if="removalNotice" role="status">{{ t(removalNotice) }}</p>
    <p v-if="!supported" class="muted">{{ t("service.catalogStorage.unsupported") }}</p>
    <n-alert v-if="error" type="error" :title="t('service.catalogStorage.inspectionIncomplete')">{{ error }}</n-alert>
    <template v-if="entries">
      <p>{{ t(managed ? 'service.catalogStorage.usage' : 'service.catalogStorage.usageWithQuota', { used: size(totalBytes), count: entries.length }) }}</p>
      <p v-if="!managed" class="muted">{{ t("service.catalogStorage.catalogHint") }}</p>
      <p v-else class="muted">{{ t("service.catalogStorage.managedHint") }}</p>
      <p v-if="entries.length === 0">{{ t("service.catalogStorage.noDirectories") }}</p>
      <template v-else>
        <n-input v-model:value="query" clearable :placeholder="t('service.catalogStorage.filterPlaceholder')" :aria-label="t('service.catalogStorage.filterLabel')" />
        <p v-if="filtered.length === 0">{{ t("service.catalogStorage.noMatches") }}</p>
        <n-collapse v-else class="storage-entries">
          <n-collapse-item v-for="entry in visible" :key="entry.directory" :name="entry.directory">
            <template #header><span class="directory">{{ entry.directory }}</span></template>
            <template #header-extra>{{ size(entry.bytes) }}</template>
            <p>{{ status(entry) }}</p>
            <p v-if="entry.issue">{{ backendText(entry.issue, entry.issue_ui) }}</p>
            <p v-if="entry.runtime_referenced">{{ t("service.catalogStorage.runtimeReferenced") }}</p>
            <p v-if="entry.saved_references?.length">{{ t("service.catalogStorage.savedReferences", { count: entry.saved_reference_count ?? entry.saved_references.length }) }}</p>
            <ul v-if="entry.saved_references?.length"><li v-for="source in entry.saved_references" :key="source">{{ source }}</li></ul>
            <p v-if="(entry.saved_reference_count ?? 0) > (entry.saved_references?.length ?? 0)">{{ t("service.catalogStorage.moreSources") }}</p>
            <n-button v-if="entry.ownership_verified && daemon.info?.capabilities.includes(managed ? 'managed_media_review' : 'catalog_cleanup_review')" :disabled="busy" @click="reviewFiles(entry)">{{ t("service.catalogStorage.reviewFiles") }}</n-button>
          </n-collapse-item>
        </n-collapse>
        <n-pagination v-if="filtered.length > 20" v-model:page="page" :page-size="20" :item-count="filtered.length" />
      </template>
    </template>
  </n-card>
  <n-modal :show="showReview" preset="card" :title="t(managed ? 'service.catalogStorage.reviewTitleManaged' : 'service.catalogStorage.reviewTitleCatalog')" class="catalog-review" :mask-closable="false" @update:show="closeReview">
    <p v-if="busy" role="status">{{ t(removalOperation ? 'service.catalogStorage.busyRemoving' : 'service.catalogStorage.busyReading') }}</p>
    <p v-if="removalNotice && !review" role="status">{{ t(removalNotice) }}</p>
    <n-alert v-if="reviewError" type="error">{{ reviewError }}</n-alert>
    <template v-if="review">
      <p>{{ review.contents.directory }} · {{ size(review.contents.bytes) }} · {{ status(review.references) }}</p>
      <p v-if="!managed">{{ t("service.catalogStorage.reviewCatalogHint") }}</p>
      <p v-else>{{ t("service.catalogStorage.reviewManagedHint") }}</p>
      <p v-if="review.references.runtime_referenced">{{ t("service.catalogStorage.observedRuntime") }}</p>
      <p v-if="review.references.saved_reference_count">{{ t("service.catalogStorage.savedSourcesCount", { count: review.references.saved_reference_count }) }}</p>
      <ul v-if="review.references.saved_references?.length"><li v-for="source in review.references.saved_references" :key="source">{{ source }}</li></ul>
      <div class="review-files">
        <p v-if="managed && review.contents.files.length === 0">{{ t("service.catalogStorage.noRetainedFiles") }}</p>
        <ul><li v-for="file in reviewFilesPage" :key="file.path">{{ file.path }} · {{ size(file.bytes) }} · {{ t(fileStateLabel(file.matches_catalog)) }}</li></ul>
        <n-pagination v-if="review.contents.files.length > 32" v-model:page="reviewPage" :page-size="32" :item-count="review.contents.files.length" />
        <template v-if="review.contents.missing_files.length"><p>{{ t("service.catalogStorage.missingFiles", { count: review.contents.missing_files.length }) }}</p><ul><li v-for="path in missingFilesPage" :key="path">{{ path }}</li></ul><n-pagination v-if="review.contents.missing_files.length > 32" v-model:page="missingPage" :page-size="32" :item-count="review.contents.missing_files.length" /></template>
      </div>
      <n-checkbox v-if="canRemove" v-model:checked="confirmed" :disabled="busy">{{ t("service.catalogStorage.removeConfirm") }}</n-checkbox>
    </template>
    <template #footer><n-space><n-button v-if="canRemove" type="error" :disabled="busy || !confirmed" @click="removeFiles(true)">{{ t("service.catalogStorage.removeDirectory") }}</n-button><n-button @click="closeReview">{{ t("common.close") }}</n-button></n-space></template>
  </n-modal>
</template>

<style scoped>
p { margin: var(--space-3) 0; }
.muted { color: var(--text-secondary); }
.storage-entries { margin: var(--space-3) 0; }
.directory, li { overflow-wrap: anywhere; }
.directory { min-width: 0; margin-right: var(--space-3); }
.catalog-review { width: min(900px, 92vw); }
.review-files { max-height: 50vh; overflow: auto; }
</style>
