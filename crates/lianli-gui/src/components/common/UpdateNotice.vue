<script setup lang="ts">
import { onMounted } from "vue";
import { ExternalLink } from "lucide-vue-next";
import { useUpdatesStore } from "@/stores/updates";
import { useExternalLink } from "@/composables/useExternalLink";
import { useTranslate } from "@/i18n";

const updates = useUpdatesStore();
const openExternal = useExternalLink();
const t = useTranslate();

onMounted(() => updates.start());
</script>

<template>
  <n-alert v-if="updates.showNotice" type="info" closable @close="updates.dismiss()">
    <span class="update-line">
      {{ t("shell.updateNotice.title", { version: updates.available ?? "" }) }}
      <button class="release-link" @click="openExternal(`${updates.releaseUrl}/tag/v${updates.available}`)">
        <ExternalLink :size="13" /> {{ t("shell.updateNotice.releaseNotes") }}
      </button>
    </span>
  </n-alert>
</template>

<style scoped>
.update-line {
  display: inline-flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-3);
}
.release-link {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 0;
  border: none;
  background: none;
  color: var(--accent);
  font: inherit;
  cursor: pointer;
}
</style>
