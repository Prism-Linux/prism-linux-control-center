<script setup lang="ts">
import { ref, onErrorCaptured } from "vue";
import { useTranslate } from "@/i18n";

const t = useTranslate();
const error = ref(false);
const attempt = ref(0);

onErrorCaptured((cause, _instance, info) => {
  console.error("Page rendering failed", info, cause);
  error.value = true;
  return false;
});

function retry() {
  error.value = false;
  attempt.value++;
}
</script>

<template>
  <div v-if="error" class="page-error" role="alert">
    <strong>{{ t("common.pageRenderError") }}</strong>
    <p>{{ t("common.pageRenderErrorHint") }}</p>
    <n-button size="small" @click="retry">{{ t("common.retry") }}</n-button>
  </div>
  <div v-else :key="attempt" class="page-content">
    <slot />
  </div>
</template>

<style scoped>
.page-error { display: grid; justify-items: start; gap: var(--space-2); padding: var(--space-4); color: var(--text-primary); }
.page-error p { margin: 0; color: var(--text-secondary); }
.page-content { min-width: 0; min-height: 0; }
</style>
