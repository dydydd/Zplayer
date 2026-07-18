import assert from "node:assert/strict";
import test from "node:test";
import { normalizeServerIconName, parseServerIconCatalog, resolveServerIconUrl, serverIconCatalogUrls } from "./serverIcons.ts";

const icons = [
  { name: "Forward", url: "https://example.test/Forward.PNG" },
  { name: "Alpha_Pro", url: "https://example.test/Alpha_Pro.PNG" },
  { name: "Prime Video", url: "https://example.test/Prime%20Video.PNG" },
  { name: "Emby", url: "https://example.test/Emby.PNG" },
];

test("parses TFEL-style server icon catalogs", () => {
  assert.deepEqual(parseServerIconCatalog({
    name: "TFEL",
    icons: [
      { name: "Forward", url: "/icons/Forward.PNG" },
      { name: "", url: "/icons/Empty.PNG" },
      { name: "Bad", url: "file:///tmp/bad.png" },
      { label: "ignored", href: "/icons/ignored.png" },
    ],
  }, "https://emby-icon.vercel.app/TFEL-Emby.json"), [
    { name: "Forward", url: "https://emby-icon.vercel.app/icons/Forward.PNG" },
  ]);
});

test("matches server icons by normalized name", () => {
  assert.equal(normalizeServerIconName("Alpha_Pro"), "alpha pro");
  assert.equal(resolveServerIconUrl("forward", icons), "https://example.test/Forward.PNG");
  assert.equal(resolveServerIconUrl("Alpha Pro", icons), "https://example.test/Alpha_Pro.PNG");
  assert.equal(resolveServerIconUrl("PrimeVideo", icons), "https://example.test/Prime%20Video.PNG");
});

test("matches server names with common suffixes without using generic emby", () => {
  assert.equal(resolveServerIconUrl("Forward Emby", icons), "https://example.test/Forward.PNG");
  assert.equal(resolveServerIconUrl("My Emby Server", icons), null);
});

test("splits global server icon catalog urls", () => {
  assert.deepEqual(
    serverIconCatalogUrls(" https://one.example/icons.json \n\nhttps://two.example/icons.json，https://three.example/icons.json "),
    [
      "https://one.example/icons.json",
      "https://two.example/icons.json",
      "https://three.example/icons.json",
    ],
  );
});
