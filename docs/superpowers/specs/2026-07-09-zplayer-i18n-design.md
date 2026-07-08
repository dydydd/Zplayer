# Zplayer i18n

Date: 2026-07-09

## Goal

Add internationalization with Simplified Chinese and English, defaulting to the system language while allowing the user to override the language in settings.

## Current behavior

Most user-facing text is hardcoded in Chinese across React views, helpers, aria labels, titles, loading labels, and error messages. There is no current i18n library or translation resource structure.

## Approach

Use `i18next` with `react-i18next`.

- Add `i18next` and `react-i18next` frontend dependencies.
- Add `src/i18n.ts` to initialize i18next before rendering the app.
- Add local resource files:
  - `src/locales/zh-CN.ts`
  - `src/locales/en-US.ts`
- Add `language` to app settings with values `auto`, `zh-CN`, and `en-US`.
- Keep `auto` as the default. It resolves from `navigator.languages`, falls back to `navigator.language`, then falls back to `zh-CN`.
- Add a language selector to the existing settings page.
- Apply language changes immediately after saving settings.

The first implementation pass should translate the main product surface: navigation, server management, settings, home shelves, library/search filters, item details, player controls, loading labels, aria labels, and common error messages.

## Data flow

1. App loads settings through the existing `load_settings` IPC command.
2. Frontend resolves the effective language from the saved setting.
3. `i18next.changeLanguage` applies the language.
4. Components read display strings through `useTranslation` or the shared i18n instance.
5. Saving settings persists the selected language with the existing settings store.

## Boundaries

- Do not add remote translation loading.
- Do not add language detection beyond browser/system language.
- Do not split translations into many namespaces in the first pass; one compact resource per language is enough.
- Do not translate media metadata from Emby/Jellyfin.
- Keep server-provided names, people, titles, descriptions, stream labels, and external error details as source-provided text.

## Error handling

- Missing translation keys should be obvious during development. Do not silently invent fallback strings in components.
- Unsupported saved language values fall back to `auto`.
- If language detection returns an unsupported locale such as `en-GB`, match by base language and use `en-US`.

## Tests

- Unit test language normalization and browser language matching.
- TypeScript build must fail on invalid translation key usage where practical.
- Manual check language switching without app restart.
- Manual check first-run behavior with Chinese and English browser language preferences.

## References

- i18next getting started: https://www.i18next.com/overview/getting-started

## Non-goals

- No translation management service.
- No pluralization-heavy rewrite beyond strings that need counts now.
- No date, number, or media-runtime localization beyond current simple labels unless a migrated string needs it.
