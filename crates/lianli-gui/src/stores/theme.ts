import { defineStore } from "pinia";
import { computed, ref, watch } from "vue";

export type ThemeMode = "dark" | "light";
export type GlowLevel = "full" | "subtle" | "minimal";

const STORAGE_KEY = "lianli-linux.theme";
const GLOW_STORAGE_KEY = "lianli-linux.glow";
const VALID: ThemeMode[] = ["dark", "light"];
const VALID_GLOW: GlowLevel[] = ["full", "subtle", "minimal"];

function readStored(): ThemeMode | null {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    return v && VALID.includes(v as ThemeMode) ? (v as ThemeMode) : null;
  } catch {
    return null;
  }
}

function prefersLight(): boolean {
  try {
    return window.matchMedia?.("(prefers-color-scheme: light)").matches ?? false;
  } catch {
    return false;
  }
}

function readGlow(): GlowLevel {
  try {
    const value = localStorage.getItem(GLOW_STORAGE_KEY);
    return value && VALID_GLOW.includes(value as GlowLevel) ? value as GlowLevel : "full";
  } catch {
    return "full";
  }
}

/**
 * UI colour-scheme preference. Defaults to the OS setting, persists any
 * explicit user choice to localStorage, and reflects the active mode onto
 * `document.documentElement` so the CSS variable palette can switch.
 */
export const useThemeStore = defineStore("theme", () => {
  const stored = readStored();
  const mode = ref<ThemeMode>(stored ?? (prefersLight() ? "light" : "dark"));
  const glow = ref<GlowLevel>(readGlow());

  const isDark = computed(() => mode.value === "dark");

  function set(next: ThemeMode) {
    mode.value = next;
  }

  function toggle() {
    mode.value = mode.value === "dark" ? "light" : "dark";
  }

  function setGlow(next: GlowLevel) {
    glow.value = next;
  }

  function apply() {
    const el = document.documentElement;
    el.setAttribute("data-theme", mode.value);
    el.setAttribute("data-glow", glow.value);
    el.style.colorScheme = mode.value;
  }

  watch(
    mode,
    (next) => {
      try {
        localStorage.setItem(STORAGE_KEY, next);
      } catch {
        // storage unavailable — keep in-memory only
      }
      apply();
    },
    { immediate: true },
  );

  watch(glow, (next) => {
    try {
      localStorage.setItem(GLOW_STORAGE_KEY, next);
    } catch {
      // Keep the selected glow level for this session if storage is unavailable.
    }
    apply();
  }, { immediate: true });

  return { mode, isDark, set, toggle, glow, setGlow };
});
