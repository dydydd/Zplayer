import i18next from "i18next";
import { initReactI18next } from "react-i18next";
import { effectiveLanguage, type AppLanguage } from "./i18nLogic";
import { enUS } from "./locales/en-US";
import { zhCN } from "./locales/zh-CN";

export function browserLanguageCandidates() {
  if (typeof navigator === "undefined") return [];
  return [...(navigator.languages ?? []), navigator.language].filter(Boolean);
}

void i18next.use(initReactI18next).init({
  resources: {
    "zh-CN": { translation: zhCN },
    "en-US": { translation: enUS },
  },
  lng: effectiveLanguage("auto", browserLanguageCandidates()),
  fallbackLng: "zh-CN",
  interpolation: { escapeValue: false },
});

export function applyLanguage(setting: AppLanguage) {
  return i18next.changeLanguage(effectiveLanguage(setting, browserLanguageCandidates()));
}

export default i18next;
