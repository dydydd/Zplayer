import assert from "node:assert/strict";
import test from "node:test";
import { subtitleDialogFilters } from "./subtitleDialog.ts";

test("uses one native dialog filter for common subtitle files", () => {
  assert.deepEqual(subtitleDialogFilters, [
    {
      name: "Subtitles",
      extensions: ["srt", "ass", "ssa", "vtt", "sub", "idx"],
    },
  ]);
});
