import { createI18n, useI18n } from "vue-i18n";
import en from "./locales/en";
import zhCN from "./locales/zh-CN";

export type MessageSchema = typeof en;

type KeyPaths<T, Prefix extends string = ""> = {
  [K in keyof T & string]: T[K] extends string ? `${Prefix}${K}` : KeyPaths<T[K], `${Prefix}${K}.`>;
}[keyof T & string];

/** Every message key; vue-i18n's own `t` accepts any string, so typos would only show at runtime. */
export type MessageKey = KeyPaths<MessageSchema>;
export type MessageParams = Record<string, unknown>;

export const SUPPORTED_LOCALES = ["en", "zh-CN"] as const;
export type AppLocale = (typeof SUPPORTED_LOCALES)[number];

declare module "vue-i18n" {
  export interface DefineLocaleMessage extends MessageSchema {}
}

export const i18n = createI18n<[MessageSchema], AppLocale, false>({
  legacy: false,
  locale: "en",
  fallbackLocale: "en",
  messages: { en, "zh-CN": zhCN },
});

/** Translate outside components; call it where the result is read so locale changes apply. */
export function tr(key: MessageKey, params: MessageParams = {}): string {
  return i18n.global.t(key, params);
}

/** Component translation with checked keys. */
export function useTranslate() {
  const { t } = useI18n();
  return (key: MessageKey, params: MessageParams = {}): string => t(key, params);
}
