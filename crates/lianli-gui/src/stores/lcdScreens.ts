import { defineStore } from "pinia";
import { ref } from "vue";
import { readStoredJson, writeStoredJson } from "@/utils/storedPreference";

const ORDER_KEY = "lianli-linux.lcdOrder";

function readOrder(): string[] {
  const stored = readStoredJson(ORDER_KEY);
  return Array.isArray(stored) ? stored.filter((key): key is string => typeof key === "string") : [];
}

export const useLcdScreensStore = defineStore("lcdScreens", () => {
  const order = ref<string[]>(readOrder());

  function setOrder(keys: string[]) {
    order.value = [...keys];
    writeStoredJson(ORDER_KEY, order.value);
  }

  function carryOver(from: string, to: string) {
    if (from === to) return;
    const position = order.value.indexOf(from);
    if (position !== -1 && !order.value.includes(to)) {
      setOrder([...order.value.slice(0, position + 1), to, ...order.value.slice(position + 1)]);
    }
  }

  return { order, setOrder, carryOver };
});
