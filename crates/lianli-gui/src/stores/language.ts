import { defineStore } from "pinia";
import { computed, ref, watch } from "vue";
import { i18n, SUPPORTED_LOCALES, type AppLocale } from "@/i18n";

export type LanguagePreference = "system" | AppLocale;

const STORAGE_KEY = "lianli-linux.language";

function parse(value: string | null): LanguagePreference | null {
  if (value === "system") return "system";
  return SUPPORTED_LOCALES.find((locale) => locale === value) ?? null;
}

function readStored(): LanguagePreference | null {
  try {
    return parse(localStorage.getItem(STORAGE_KEY));
  } catch {
    return null;
  }
}

function systemLocale(): AppLocale {
  const preferred = navigator.languages?.[0] ?? navigator.language ?? "";
  return preferred.toLowerCase().startsWith("zh") ? "zh-CN" : "en";
}

export const useLanguageStore = defineStore("language", () => {
  const preference = ref<LanguagePreference>(readStored() ?? "system");
  const locale = computed<AppLocale>(() =>
    preference.value === "system" ? systemLocale() : preference.value,
  );

  function set(next: LanguagePreference) {
    preference.value = next;
    try {
      localStorage.setItem(STORAGE_KEY, next);
    } catch {
      // storage unavailable — keep in-memory only
    }
  }

  watch(
    locale,
    (next) => {
      i18n.global.locale.value = next;
      document.documentElement.lang = next;
    },
    { immediate: true },
  );

  // The editor and browser windows share storage; follow changes made in another window.
  window.addEventListener("storage", (event) => {
    if (event.key === STORAGE_KEY) preference.value = parse(event.newValue) ?? "system";
  });

  return { preference, locale, set };
});
