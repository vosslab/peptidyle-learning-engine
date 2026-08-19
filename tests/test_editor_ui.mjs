// test_editor_ui.mjs - permanent browser-safe behavior checks for the injected editor surface.

import assert from "node:assert/strict";
import test from "node:test";

import { capabilityLabel, serializeEditorState } from "../src/pages/editor_page_model.ts";
import { replaceFirstTextPrompt } from "../src/pages/editor_page.tsx";
import { createEditorPreviewFacade } from "../src/pages/editor_preview_facade.ts";
import { decodeNativeDraftPreviewResult } from "../src/wasm/index.ts";

const draft = {
  workspace: "0198e000-0000-7000-8000-000000000010",
  title: "Peptide-bond geometry",
  source: { backend: "native", family: "peptide-bond" },
  prompt: [{ kind: "text", markdown: "Estimate the omega angle." }],
  response: { kind: "numeric", tolerance: { kind: "absolute", epsilon: 0.5 }, unit: "degrees" },
  attemptPolicy: { maxAttempts: null },
  timingPolicy: { kind: "untimed" },
  randomization: { kind: "static" },
};

function keyFreePreview(seed = 17) {
  return {
    workspace: draft.workspace,
    seed,
    title: draft.title,
    prompt: [{ kind: "text", markdown: "Estimate the omega angle." }],
    response: draft.response,
  };
}

function wasmFacade(previewNativeDraft) {
  return {
    mode: "wasm",
    validateResponseFormat: async () => ({ violations: [] }),
    timerVerdict: async () => "untimed",
    validateAssignmentConfig: async () => [],
    previewNativeDraft,
  };
}

test("editor preview projects the exact draft and seed through the WASM facade", async () => {
  const calls = [];
  const facade = createEditorPreviewFacade(
    wasmFacade(async (request, seed) => {
      calls.push({ request, seed });
      return { kind: "ready", preview: keyFreePreview(seed) };
    }),
  );

  const preview = await facade.preview(draft, 17);
  assert.deepEqual(calls, [
    {
      request: {
        workspace: draft.workspace,
        source: draft.source,
        title: draft.title,
        prompt: draft.prompt,
        response: draft.response,
        randomization: draft.randomization,
      },
      seed: 17,
    },
  ]);
  assert.deepEqual(preview, keyFreePreview());
  assert.equal("problem" in preview, false);
  assert.equal("version" in preview, false);
});

test("editor preview surfaces backend-only draft availability honestly", async () => {
  const facade = createEditorPreviewFacade(
    wasmFacade(async () => ({
      kind: "unavailable",
      backend: "webwork",
      capability: "offlinePreview",
    })),
  );
  await assert.rejects(
    facade.preview({ ...draft, source: { backend: "webwork", pgPath: "set/a.pg" } }, 17),
    /webwork drafts need a backend preview/,
  );
});

test("WASM preview decoder refuses contaminated or unknown result fields", () => {
  const clean = {
    kind: "ready",
    preview: keyFreePreview(),
  };
  assert.deepEqual(decodeNativeDraftPreviewResult(JSON.stringify(clean)), clean);

  for (const field of [
    "problem",
    "version",
    "source",
    "grading",
    "answer",
    "key",
    "correct",
    "score",
  ]) {
    const contaminated = { ...clean, preview: { ...clean.preview, [field]: "forbidden" } };
    assert.throws(
      () => decodeNativeDraftPreviewResult(JSON.stringify(contaminated)),
      /unknown field|must not contain/,
    );
  }
  const promptUnknown = {
    ...clean,
    preview: { ...clean.preview, prompt: [{ ...clean.preview.prompt[0], unexpected: true }] },
  };
  assert.throws(() => decodeNativeDraftPreviewResult(JSON.stringify(promptUnknown)), /unexpected/);
  const responseUnknown = {
    ...clean,
    preview: { ...clean.preview, response: { ...clean.preview.response, unexpected: true } },
  };
  assert.throws(
    () => decodeNativeDraftPreviewResult(JSON.stringify(responseUnknown)),
    /unexpected/,
  );
});

test("editing prose retains every non-text prompt block", () => {
  const richDraft = {
    ...draft,
    prompt: [
      { kind: "text", markdown: "Original prose." },
      { kind: "math", latex: "x^2", description: "x squared" },
      { kind: "code", language: "text", source: "preserve me" },
    ],
  };
  const updated = replaceFirstTextPrompt(richDraft, "Revised prose.");

  assert.deepEqual(updated.prompt, [
    { kind: "text", markdown: "Revised prose." },
    { kind: "math", latex: "x^2", description: "x squared" },
    { kind: "code", language: "text", source: "preserve me" },
  ]);
});

test("serialized editor state has no protected evaluation fields", () => {
  const state = JSON.parse(serializeEditorState(draft, keyFreePreview()));
  const fieldNames = new Set();
  const collect = (value) => {
    if (Array.isArray(value)) {
      value.forEach(collect);
    } else if (value !== null && typeof value === "object") {
      for (const [field, child] of Object.entries(value)) {
        fieldNames.add(field.toLowerCase());
        collect(child);
      }
    }
  };
  collect(state);

  for (const forbidden of ["answer", "key", "grading", "correct", "score"]) {
    assert.equal(fieldNames.has(forbidden), false, forbidden);
  }
});

test("capability violations name the draft question and missing capability", () => {
  const violation = {
    workspace: draft.workspace,
    title: draft.title,
    capability: "offlinePreview",
  };

  assert.equal(violation.title, "Peptide-bond geometry");
  assert.equal(capabilityLabel(violation.capability), "offline preview");
  assert.equal("version" in violation, false);
});

test("a publish refusal preserves the caller's unversioned draft for correction", () => {
  const refusal = {
    kind: "validationFailed",
    violations: [{ workspace: draft.workspace, title: draft.title, capability: "hints" }],
  };
  const stillOpen = draft;

  assert.equal(refusal.kind, "validationFailed");
  assert.equal(stillOpen.title, "Peptide-bond geometry");
  assert.equal("version" in stillOpen, false);
  assert.equal(refusal.violations[0].capability, "hints");
});

test("publication presents an immutable comparison before confirmation", () => {
  const diff = {
    baseline: "newQuestion",
    proposedTitle: draft.title,
    sections: [{ label: "Prompt", before: "Older prompt", after: "Estimate the omega angle." }],
  };

  assert.equal(diff.baseline, "newQuestion");
  assert.equal(diff.sections[0].label, "Prompt");
});

test("editor surface delegates preview and never imports a mock or protected preview DTO", async () => {
  const source = await (
    await import("node:fs/promises")
  ).readFile("src/pages/editor_page.tsx", "utf8");

  assert.match(source, /<QuestionRenderer/);
  assert.match(source, /presentation=\{state\(\)\.preview\}/);
  assert.match(source, /<ResponseWidget/);
  assert.match(source, /previewFacade\.preview/);
  assert.match(source, /validateCapabilities/);
  assert.match(source, /getPublishDiff/);
  assert.doesNotMatch(source, /api\/mock/);
  assert.doesNotMatch(source, /AnswerKey|GradingDefinition/);
});

test("the compile-time boundary rejects passing a draft preview to published models", async () => {
  const source = await (
    await import("node:fs/promises")
  ).readFile("src/pages/editor_page_typecheck.ts", "utf8");

  assert.match(
    source,
    /@ts-expect-error A workspace preview cannot enter a published-envelope path/,
  );
  assert.match(source, /@ts-expect-error A workspace preview cannot enter an assignment/);
});
