export type AppLanguage = "auto" | "zh-CN" | "en-US";
export type SupportedLanguage = Exclude<AppLanguage, "auto">;

export const supportedLanguages = ["zh-CN", "en-US"] as const satisfies readonly SupportedLanguage[];

export function normalizeLanguage(value?: string | null): AppLanguage {
  return value === "zh-CN" || value === "en-US" ? value : "auto";
}

export function resolveAutoLanguage(languages: readonly string[] = []): SupportedLanguage {
  for (const language of languages) {
    const normalized = language.toLowerCase();
    const base = normalized.split("-")[0];
    if (normalized === "zh-cn" || base === "zh") return "zh-CN";
    if (normalized === "en-us" || base === "en") return "en-US";
  }
  return "zh-CN";
}

export function effectiveLanguage(setting?: string | null, browserLanguages: readonly string[] = []): SupportedLanguage {
  const language = normalizeLanguage(setting);
  return language === "auto" ? resolveAutoLanguage(browserLanguages) : language;
}
