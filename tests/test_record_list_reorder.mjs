import assert from "node:assert/strict";

import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";
import { createComponent } from "solid-js";
import { renderToString } from "solid-js/web";

import {
  nativeDragDestinationIndex,
  nativeDragIndex,
  reorderedRecordListRows,
} from "../src/components/record_list/record_list_reorder.tsx";
import { reordered } from "../src/features/blueprint_forks/blueprint_fork_apply_model.ts";

const rows = ["alpha", "bravo", "charlie"];

assert.deepEqual(reorderedRecordListRows(rows, 1, 0), ["bravo", "alpha", "charlie"]);
assert.deepEqual(reorderedRecordListRows(rows, 0, 2), ["bravo", "charlie", "alpha"]);
assert.deepEqual(reorderedRecordListRows(rows, -1, 1), rows);
assert.deepEqual(reorderedRecordListRows(rows, 1, 3), rows);
assert.deepEqual(reordered(rows, 2, -1), ["alpha", "charlie", "bravo"]);
assert.deepEqual(reordered(rows, 0, -1), rows);

const alphaOntoCharlie = nativeDragDestinationIndex(rows, "alpha", "charlie", () => false);
assert.equal(alphaOntoCharlie, 2);
assert.deepEqual(
  reorderedRecordListRows(rows, rows.indexOf("alpha"), alphaOntoCharlie),
  ["bravo", "charlie", "alpha"],
  "native drag moves Alpha onto Charlie",
);
assert.equal(
  nativeDragDestinationIndex(rows, "alpha", "alpha", () => false),
  undefined,
);
assert.equal(
  nativeDragDestinationIndex(rows, "alpha", "missing", () => false),
  undefined,
);
assert.equal(
  nativeDragDestinationIndex(rows, "alpha", "charlie", (recordId) => recordId === "alpha"),
  undefined,
);
assert.equal(
  nativeDragDestinationIndex(rows, "alpha", "charlie", (recordId) => recordId === "charlie"),
  undefined,
);
assert.equal(nativeDragIndex(0, 2, rows.length, false), 2);
assert.equal(nativeDragIndex(1, 1, rows.length, false), undefined);
assert.equal(nativeDragIndex(0, 2, rows.length, true), undefined);

const reorderBundle = await build({
  bundle: true,
  entryPoints: [
    new URL("../src/components/record_list/record_list_reorder.tsx", import.meta.url).pathname,
  ],
  format: "esm",
  outfile: "record_list_reorder.js",
  platform: "node",
  plugins: [solidPlugin({ solid: { generate: "ssr", hydratable: false } })],
  write: false,
});
const reorderJavaScript = reorderBundle.outputFiles.find((output) => output.path.endsWith(".js"));
if (reorderJavaScript === undefined)
  throw new Error("Record reorder bundle is missing JavaScript.");
const reorderModule = await import(
  `data:text/javascript;base64,${Buffer.from(reorderJavaScript.contents).toString("base64")}`
);
if (typeof reorderModule.RecordMoveControls !== "function") {
  throw new Error("Record reorder bundle does not export RecordMoveControls.");
}

function renderMoveControls(disabled) {
  return renderToString(() =>
    createComponent(reorderModule.RecordMoveControls, {
      recordLabel: "Alpha",
      index: () => 0,
      count: () => rows.length,
      disabled: () => disabled,
      move: () => {},
      startDrag: () => {},
      allowDrop: () => {},
      completeDrop: () => {},
      endDrag: () => {},
    }),
  );
}

const enabledDrag = renderMoveControls(false);
assert.match(enabledDrag, /Drag to reorder/);
assert.match(enabledDrag, /Drag Alpha to a new position/);
assert.match(enabledDrag, /draggable="true"/);
const disabledDrag = renderMoveControls(true);
assert.match(disabledDrag, /draggable="false"/);

process.stdout.write("RecordList pure reorder helper checks passed.\n");
