import { createI18n } from "vue-i18n";
import de from "./locales/de";
import en from "./locales/en";

export type Locale = "de" | "en";

const stored = localStorage.getItem("locale");
const initial: Locale = stored === "en" || stored === "de" ? stored : "de";

export const i18n = createI18n({
  legacy: false,
  locale: initial,
  fallbackLocale: "en",
  messages: { de, en },
});

export function setLocale(locale: Locale) {
  i18n.global.locale.value = locale;
  localStorage.setItem("locale", locale);
  document.documentElement.lang = locale;
}

document.documentElement.lang = initial;
