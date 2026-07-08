# Complete Approved Specs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the approved cross-platform mpv playback, Linux Wayland UI smoothness, and i18next i18n specs.

**Architecture:** Keep each spec as a separately verified and committed feature. Backend platform decisions stay in small Rust helpers with unit tests. Frontend i18n uses i18next resources and existing settings persistence.

**Tech Stack:** Tauri 2, Rust, React 19, TypeScript, i18next, react-i18next, existing JSON settings store.

---

### Task 1: Cross-platform mpv playback

**Files:**
- Modify: `src-tauri/src/mpv.rs`

- [ ] **Step 1: Add failing unit coverage**

Add tests in `src-tauri/src/mpv.rs` for:

```rust
#[test]
fn default_mpv_executable_matches_current_platform() {
    #[cfg(target_os = "windows")]
    assert_eq!(default_mpv_executable_name(), "mpv.exe");

    #[cfg(not(target_os = "windows"))]
    assert_eq!(default_mpv_executable_name(), "mpv");
}

#[cfg(not(target_os = "windows"))]
#[test]
fn non_windows_embedding_is_noop() {
    assert!(platform_supports_embedding() == false);
}
```

- [ ] **Step 2: Verify red**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib mpv::tests::default_mpv_executable_matches_current_platform`

Expected: FAIL because `default_mpv_executable_name` is missing.

- [ ] **Step 3: Implement the minimal platform helpers**

Add `default_mpv_executable_name`, `mpv_not_found_message`, and `platform_supports_embedding`. Use `mpv.exe` on Windows and `mpv` elsewhere.

- [ ] **Step 4: Let non-Windows launch external mpv**

Change non-Windows `add_embed_args` from an error to `Ok(())`. Keep Windows embedding unchanged.

- [ ] **Step 5: Update mpv discovery**

Use platform executable name in default candidates. On non-Windows, return `PathBuf::from("mpv")` if no bundled executable is found so `Command::new("mpv")` can resolve through `PATH`; use the clearer not-found message only when spawn fails because the command is missing.

- [ ] **Step 6: Verify and commit**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib mpv
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

Commit:

```bash
git add src-tauri/src/mpv.rs
git commit -m "fix: launch mpv on linux and macos"
```

### Task 2: Linux Wayland UI smoothness

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/src/lib.rs`
- Create: `src-tauri/src/platform_window.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/models/input.rs`
- Modify: `src/ipc.ts`
- Modify: `src/types.ts`
- Modify: `src/serverViews.tsx`

- [ ] **Step 1: Add failing Rust tests**

Create `src-tauri/src/platform_window.rs` with test-only helpers first:

```rust
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct LinuxSessionDiagnostics {
    pub(crate) xdg_session_type: Option<String>,
    pub(crate) wayland_display_set: bool,
    pub(crate) gdk_backend: Option<String>,
    pub(crate) opaque_window: bool,
}

pub(crate) fn is_wayland_session(
    xdg_session_type: Option<&str>,
    wayland_display: Option<&str>,
) -> bool {
    xdg_session_type == Some("wayland") || wayland_display.is_some_and(|value| !value.is_empty())
}
```

Tests:

```rust
#[test]
fn detects_wayland_from_session_type_or_display() {
    assert!(is_wayland_session(Some("wayland"), None));
    assert!(is_wayland_session(None, Some("wayland-0")));
    assert!(!is_wayland_session(Some("x11"), None));
}
```

- [ ] **Step 2: Verify red**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib platform_window`

Expected: FAIL until the module is wired into `lib.rs`.

- [ ] **Step 3: Create main window manually**

Set `"create": false` on the configured main window. Add `platform_window::create_main_window(app)` in `setup` before running the app. On Linux Wayland, call `.transparent(false).background_color(tauri::utils::config::Color(5, 5, 5, 255))`; otherwise use `WebviewWindowBuilder::from_config`.

- [ ] **Step 4: Expose diagnostics**

Add a `linux_window_diagnostics` Tauri command returning `LinuxSessionDiagnostics`, add `ipc.linuxWindowDiagnostics`, and show it in the existing diagnostics section when available.

- [ ] **Step 5: Verify and commit**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib platform_window
cargo test --manifest-path src-tauri/Cargo.toml --lib
npm run build
```

Commit:

```bash
git add src-tauri/tauri.conf.json src-tauri/src/lib.rs src-tauri/src/platform_window.rs src-tauri/src/commands.rs src-tauri/src/models/input.rs src/ipc.ts src/types.ts src/serverViews.tsx
git commit -m "fix: reduce wayland window composition cost"
```

### Task 3: i18next i18n

**Files:**
- Modify: `package.json`
- Modify: `package-lock.json`
- Modify: `src/main.tsx`
- Create: `src/i18n.ts`
- Create: `src/i18nLogic.ts`
- Create: `src/i18nLogic.test.ts`
- Create: `src/locales/zh-CN.ts`
- Create: `src/locales/en-US.ts`
- Modify: `src/types.ts`
- Modify: `src-tauri/src/models/server.rs`
- Modify: `src-tauri/src/models/mod.rs`
- Modify: `src/App.tsx`
- Modify: `src/ServerModal.tsx`
- Modify: `src/TopBar.tsx`
- Modify: `src/homeView.tsx`
- Modify: `src/serverViews.tsx`
- Modify: `src/detailViews.tsx`
- Modify: `src/libraryViewsCustom.tsx`
- Modify: `src/viewParts.tsx`
- Modify: `src/media.ts`
- Modify: `src/viewLogic.ts`

- [ ] **Step 1: Install dependencies**

Run: `npm install i18next react-i18next`

- [ ] **Step 2: Add failing frontend language tests**

Add `src/i18nLogic.test.ts` using Node's built-in test runner for locale normalization:

```ts
import assert from "node:assert/strict";
import test from "node:test";
import { normalizeLanguage, resolveAutoLanguage } from "./i18nLogic.ts";

test("normalizes supported languages", () => {
  assert.equal(normalizeLanguage("zh-CN"), "zh-CN");
  assert.equal(normalizeLanguage("en-US"), "en-US");
  assert.equal(normalizeLanguage("nope"), "auto");
});

test("matches browser language by exact or base language", () => {
  assert.equal(resolveAutoLanguage(["en-GB"]), "en-US");
  assert.equal(resolveAutoLanguage(["zh-Hans-CN"]), "zh-CN");
  assert.equal(resolveAutoLanguage(["fr-FR"]), "zh-CN");
});
```

- [ ] **Step 3: Verify red**

Run: `node --test src/i18nLogic.test.ts`

Expected: FAIL because `src/i18nLogic.ts` is missing.

- [ ] **Step 4: Implement i18n setup**

Add `src/i18nLogic.ts`, `src/i18n.ts`, and locale resources. Import `./i18n` before rendering `App`.

- [ ] **Step 5: Persist language setting**

Add `language` to Rust `AppSettings` and frontend settings types. Normalize to `auto`, `zh-CN`, or `en-US`.

- [ ] **Step 6: Migrate main UI strings**

Replace hardcoded main-surface Chinese strings with `useTranslation` calls in the listed React files. Keep server-provided metadata unchanged.

- [ ] **Step 7: Verify and commit**

Run:

```bash
node --test src/i18nLogic.test.ts
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --lib models
```

Commit:

```bash
git add package.json package-lock.json src src-tauri/src/models/server.rs src-tauri/src/models/mod.rs
git commit -m "feat: add i18next localization"
```

### Task 4: Final audit

- [ ] **Step 1: Verify all specs**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib
npm run build
git status --short
```

- [ ] **Step 2: Audit spec requirements**

Compare the three spec files against current code and note any incomplete items.
