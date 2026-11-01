<script setup lang="ts">
import { computed } from "vue";
import { useDaemonStore } from "@/stores/daemon";
import { useTranslate, type MessageKey } from "@/i18n";
import type { WirelessDongle } from "@/types";

const daemon = useDaemonStore();
const t = useTranslate();

const MESSAGES: Record<WirelessDongle, { title: MessageKey; body: MessageKey }> = {
  transmitter: { title: "shell.wirelessNotice.transmitterTitle", body: "shell.wirelessNotice.transmitterBody" },
  receiver: { title: "shell.wirelessNotice.receiverTitle", body: "shell.wirelessNotice.receiverBody" },
};

const message = computed(() => (daemon.connected && daemon.unresponsiveDongle ? MESSAGES[daemon.unresponsiveDongle] : null));
</script>

<template>
  <n-alert v-if="message" type="error" :title="t(message.title)">
    {{ t(message.body) }}
  </n-alert>
</template>
