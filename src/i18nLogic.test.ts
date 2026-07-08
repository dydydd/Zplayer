import assert from "node:assert/strict";
import test from "node:test";
import { normalizeLanguage, resolveAutoLanguage } from "./i18nLogic.ts";

test("normalizes supported languages", () => {
  assert.equal(normalizeLanguage("zh-CN"), "zh-CN");
  assert.equal(normalizeLanguage("en-US"), "en-US");
  assert.equal(normalizeLanguage("nope"), "auto");
  assert.equal(normalizeLanguage(undefined), "auto");
});

test("matches browser language by exact or base language", () => {
  assert.equal(resolveAutoLanguage(["en-GB"]), "en-US");
  assert.equal(resolveAutoLanguage(["zh-Hans-CN"]), "zh-CN");
  assert.equal(resolveAutoLanguage(["fr-FR"]), "zh-CN");
});
