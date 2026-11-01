<script setup lang="ts">
import { computed } from "vue";
import { ChevronUp, ChevronDown, Plus, Trash2, Copy } from "lucide-vue-next";
import type { Widget } from "@/types";
import { useTranslate, type MessageKey } from "@/i18n";

const props = defineProps<{
  widgets: Widget[];
  selectedId: string | null;
}>();

const emit = defineEmits<{
  select: [id: string];
  add: [kindId: string];
  remove: [id: string];
  duplicate: [id: string];
  move: [id: string, dir: -1 | 1];
}>();

const t = useTranslate();

const KINDS: { id: string; label: MessageKey }[] = [
  { id: "label", label: "editor.kinds.label" },
  { id: "value_text", label: "editor.kinds.valueText" },
  { id: "radial_gauge", label: "editor.kinds.radialGauge" },
  { id: "vertical_bar", label: "editor.kinds.verticalBar" },
  { id: "horizontal_bar", label: "editor.kinds.horizontalBar" },
  { id: "speedometer", label: "editor.kinds.speedometer" },
  { id: "core_bars", label: "editor.kinds.coreBars" },
  { id: "image", label: "editor.kinds.image" },
  { id: "video", label: "editor.kinds.video" },
  { id: "clock_digital", label: "editor.kinds.clockDigital" },
  { id: "clock_analog", label: "editor.kinds.clockAnalog" },
  { id: "sparkline", label: "editor.kinds.sparkline" },
];

const kindOptions = computed(() => KINDS.map((k) => ({ label: t(k.label), value: k.id })));

function friendly(kind: any): string {
  const type = kind?.type;
  const found = KINDS.find((k) => k.id === type);
  return found ? t(found.label) : type ?? t("editor.widgetList.widget");
}
</script>

<template>
  <div class="widget-list">
    <div class="add-row">
      <n-select
        size="small"
        :placeholder="t('editor.widgetList.addWidget')"
        :options="kindOptions"
        @update:value="(v) => v && emit('add', v)"
      />
    </div>
    <div class="rows">
      <div
        v-for="(w, i) in widgets"
        :key="w.id"
        class="row"
        :class="{ selected: w.id === selectedId }"
        @click="emit('select', w.id)"
      >
        <span class="label">{{ friendly(w.kind) }}</span>
        <span class="id mono">{{ w.id }}</span>
        <div class="ops">
          <button class="op" :disabled="i === 0" @click.stop="emit('move', w.id, -1)"><ChevronUp :size="13" /></button>
          <button class="op" :disabled="i === widgets.length - 1" @click.stop="emit('move', w.id, 1)"><ChevronDown :size="13" /></button>
          <button class="op" :title="t('editor.widgetList.duplicate')" @click.stop="emit('duplicate', w.id)"><Copy :size="13" /></button>
          <button class="op danger" :title="t('editor.widgetList.delete')" @click.stop="emit('remove', w.id)"><Trash2 :size="13" /></button>
        </div>
      </div>
      <div v-if="!widgets.length" class="empty muted">{{ t("editor.widgetList.empty") }}</div>
    </div>
    <div class="hints muted">
      {{ t("editor.widgetList.hintNudge") }}<br />
      {{ t("editor.widgetList.hintShortcuts") }}
    </div>
  </div>
</template>

<style scoped>
.widget-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  height: 100%;
}
.add-row {
  flex-shrink: 0;
}
.rows {
  overflow-y: auto;
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}
.row {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  padding: var(--space-2);
  border-radius: var(--radius-sm);
  cursor: pointer;
  border: 1px solid transparent;
}
.row:hover {
  background: var(--bg-hover);
}
.row.selected {
  background: var(--accent-soft);
  border-color: var(--accent);
}
.label {
  flex: 1;
  font-size: var(--font-size-sm);
}
.id {
  font-size: var(--font-size-xs);
  color: var(--text-muted);
}
.ops {
  display: flex;
  gap: 2px;
}
.op {
  background: transparent;
  border: none;
  color: var(--text-secondary);
  cursor: pointer;
  padding: 2px;
  display: flex;
  border-radius: var(--radius-sm);
  transition: background 0.1s, color 0.1s;
}
.op:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}
.op:disabled {
  opacity: 0.3;
  cursor: default;
}
.op:disabled:hover {
  background: transparent;
  color: var(--text-secondary);
}
.op.danger:hover {
  background: rgba(248, 113, 113, 0.15);
  color: var(--danger);
}
.empty {
  padding: var(--space-3);
  font-size: var(--font-size-sm);
}
.hints {
  font-size: var(--font-size-xs);
  line-height: 1.5;
  padding: var(--space-2);
  border-top: 1px solid var(--border);
  margin-top: var(--space-2);
}
</style>
