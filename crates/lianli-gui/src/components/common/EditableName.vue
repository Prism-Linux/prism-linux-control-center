<script setup lang="ts">
import { computed, ref } from "vue";
import { Pencil } from "lucide-vue-next";
import { MAX_DEVICE_NAME_LENGTH } from "@/stores/deviceNames";
import { useTranslate } from "@/i18n";

const props = withDefaults(
  defineProps<{
    name: string;
    label: string;
    emptyLabel?: string;
    renameTitle?: string;
  }>(),
  { emptyLabel: "" },
);
const emit = defineEmits<{ rename: [name: string] }>();
const t = useTranslate();
const renameLabel = computed(() => props.renameTitle ?? t("shell.editableName.renameDevice"));

const editing = ref(false);
const draft = ref("");

function start() {
  draft.value = props.name;
  editing.value = true;
}

function commit() {
  if (!editing.value) return;
  editing.value = false;
  emit("rename", draft.value);
}
</script>

<template>
  <span class="editable-name">
    <n-input
      v-if="editing"
      v-model:value="draft"
      size="small"
      class="name-input"
      :placeholder="label || t('shell.editableName.deviceName')"
      :maxlength="MAX_DEVICE_NAME_LENGTH"
      autofocus
      @blur="commit"
      @keydown.enter="commit"
      @keydown.esc="editing = false"
    />
    <template v-else>
      <span class="name">{{ name || label || emptyLabel }}</span>
      <button type="button" class="rename-btn" :title="renameLabel" :aria-label="renameLabel" @click="start">
        <Pencil :size="12" />
      </button>
      <span v-if="name && label" class="original">{{ label }}</span>
    </template>
  </span>
</template>

<style scoped>
.editable-name {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
}
.name {
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.name-input {
  width: 220px;
}
.rename-btn {
  flex-shrink: 0;
  display: inline-flex;
  padding: 2px;
  border: none;
  border-radius: var(--radius-sm);
  background: none;
  color: var(--text-muted);
  cursor: pointer;
}
.rename-btn:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}
.original {
  font-size: var(--font-size-xs);
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
