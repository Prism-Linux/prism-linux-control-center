<script setup lang="ts">
import { computed, ref, watch } from "vue";
import SensorSelect from "@/components/common/SensorSelect.vue";
import { useTranslate } from "@/i18n";
import {
  formatSensorLabel,
  inferSensorCategory,
  SENSOR_CATEGORY_LABELS,
  sourceKey,
  sourceToConfig,
  type SelectOption,
} from "@/stores/sensorOptions";
import type { SensorBindings, SensorCategory, SensorInfo, SensorSourceConfig } from "@/types";

export interface SensorBindingRow {
  category: SensorCategory;
  suggested: SensorSourceConfig | null;
}

export interface SensorBindingRequest {
  templateName: string;
  rows: SensorBindingRow[];
  sensors: SensorInfo[];
}

const props = defineProps<{ request: SensorBindingRequest | null }>();
const emit = defineEmits<{ confirm: [bindings: SensorBindings]; cancel: [] }>();
const t = useTranslate();

const UNAVAILABLE = "";
const selected = ref<Partial<Record<SensorCategory, string>>>({});

const sources = computed(() => {
  const byKey = new Map<string, SensorSourceConfig>();
  for (const sensor of props.request?.sensors ?? []) {
    const config = sourceToConfig(sensor.source);
    byKey.set(sourceKey(config), config);
  }
  return byKey;
});

watch(
  () => props.request,
  (request) => {
    selected.value = Object.fromEntries(
      (request?.rows ?? []).map((row) => [row.category, row.suggested ? sourceKey(row.suggested) : UNAVAILABLE]),
    );
  },
  { immediate: true },
);

function optionsFor(row: SensorBindingRow): SelectOption[] {
  const matching: SelectOption[] = [];
  const other: SelectOption[] = [];
  for (const sensor of props.request?.sensors ?? []) {
    const config = sourceToConfig(sensor.source);
    const option = { label: formatSensorLabel(sensor), value: sourceKey(config) };
    (inferSensorCategory(config) === row.category ? matching : other).push(option);
  }
  const suggested = row.suggested ? sourceKey(row.suggested) : "";
  if (suggested && ![...matching, ...other].some((option) => option.value === suggested)) {
    matching.unshift({ label: t("editor.sensorBinding.detected"), value: suggested });
  }
  return [{ label: t("editor.sensorBinding.unavailable"), value: UNAVAILABLE }, ...matching, ...other];
}

function confirm() {
  const request = props.request;
  if (!request) return;
  const bindings: SensorBindings = {};
  for (const row of request.rows) {
    const key = selected.value[row.category] ?? UNAVAILABLE;
    bindings[row.category] = key === UNAVAILABLE ? null : (sources.value.get(key) ?? row.suggested);
  }
  emit("confirm", bindings);
}
</script>

<template>
  <n-modal
    :show="request !== null"
    preset="card"
    :title="t('editor.sensorBinding.title', { name: request?.templateName ?? '' })"
    style="width: min(560px, 95vw)"
    @update:show="(show: boolean) => { if (!show) emit('cancel'); }"
  >
    <div class="bindings">
      <p class="muted">{{ t("editor.sensorBinding.hint") }}</p>
      <div
        v-for="row in request?.rows ?? []"
        :key="row.category"
        class="row"
        :class="{ unavailable: selected[row.category] === UNAVAILABLE }"
      >
        <label :for="`sensor-binding-${row.category}`">{{ t(SENSOR_CATEGORY_LABELS[row.category]) }}</label>
        <SensorSelect
          :id="`sensor-binding-${row.category}`"
          size="small"
          :value="selected[row.category] ?? UNAVAILABLE"
          :options="optionsFor(row)"
          @update:value="(value: string) => (selected[row.category] = value)"
        />
      </div>
    </div>
    <template #footer>
      <div class="actions">
        <n-button @click="emit('cancel')">{{ t("common.cancel") }}</n-button>
        <n-button type="primary" @click="confirm">{{ t("editor.browser.install") }}</n-button>
      </div>
    </template>
  </n-modal>
</template>

<style scoped>
.bindings { display: flex; flex-direction: column; gap: var(--space-3); }
.bindings p { margin: 0; }
.row { display: grid; grid-template-columns: 150px minmax(0, 1fr); align-items: center; gap: var(--space-3); }
.row.unavailable label { color: var(--warning); }
.actions { display: flex; justify-content: flex-end; gap: var(--space-2); }
</style>
