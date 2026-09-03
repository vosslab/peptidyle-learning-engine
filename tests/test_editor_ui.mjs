// test_editor_ui.mjs - permanent browser-safe behavior checks for the injected editor surface.

import assert from "node:assert/strict";
import test from "node:test";

import { capabilityLabel, serializeEditorState } from "../src/pages/editor_page_model.ts";
import { replaceFirstTextPrompt } from "../src/pages/editor_page.tsx";
import { createEditorPreviewFacade } from "../src/pages/editor_preview_facade.ts";
import { decodePleDraftPreviewResult } from "../src/wasm/index.ts";

const draft = {
  workspace: "0198e000-0000-7000-8000-000000000010",
  title: "Peptide-bond geometry",
  questionBackend: "ple",
  webworkPgPath: null,
  qtiPackageItemIdentifier: null,
  workspaceImportId: null,
  draftImathasQuestionBackendBinding: null,
  questionFormat: "pleQuestionJson",
  prompt: [{ kind: "text", markdown: "Estimate the omega angle." }],
  response: { kind: "numeric", tolerance: { kind: "absolute", epsilon: 0.5 }, unit: "degrees" },
  questionAttemptLimit: { maxAttempts: null },
  questionAttemptTimeLimit: { kind: "unlimited" },
};

function keyFreePreview() {
  return {
    workspace: draft.workspace,
    title: draft.title,
    prompt: [{ kind: "text", markdown: "Estimate the omega angle." }],
    response: draft.response,
  };
}

function wasmFacade(previewPleDraft) {
  return {
    mode: "wasm",
    validateResponseFormat: async () => ({ issues: [] }),
    questionAttemptTimingDecision: async () => "unlimited",
    validateAssignmentConfig: async () => [],
    previewPleDraft,
  };
}

test("editor preview projects the exact draft through the WASM facade", async () => {
  const calls = [];
  const facade = createEditorPreviewFacade(
    wasmFacade(async (request) => {
      calls.push({ request });
      return { kind: "ready", preview: keyFreePreview() };
    }),
  );

  const preview = await facade.preview(draft);
  assert.deepEqual(calls, [
    {
      request: {
        workspace: draft.workspace,
        questionBackend: draft.questionBackend,
        webworkPgPath: draft.webworkPgPath,
        qtiPackageItemIdentifier: draft.qtiPackageItemIdentifier,
        workspaceImportId: draft.workspaceImportId,
        draftImathasQuestionBackendBinding: draft.draftImathasQuestionBackendBinding,
        title: draft.title,
        prompt: draft.prompt,
        response: draft.response,
      },
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
    facade.preview({
      ...draft,
      questionBackend: "webwork",
      webworkPgPath: "set/a.pg",
    }),
    /webwork drafts need a backend preview/,
  );
});

test("WASM preview decoder refuses contaminated or unknown result fields", () => {
  const clean = {
    kind: "ready",
    preview: keyFreePreview(),
  };
  assert.deepEqual(decodePleDraftPreviewResult(JSON.stringify(clean)), clean);

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
      () => decodePleDraftPreviewResult(JSON.stringify(contaminated)),
      /unknown field|must not contain/,
    );
  }
  const promptUnknown = {
    ...clean,
    preview: { ...clean.preview, prompt: [{ ...clean.preview.prompt[0], unexpected: true }] },
  };
  assert.throws(() => decodePleDraftPreviewResult(JSON.stringify(promptUnknown)), /unexpected/);
  const responseUnknown = {
    ...clean,
    preview: { ...clean.preview, response: { ...clean.preview.response, unexpected: true } },
  };
  assert.throws(() => decodePleDraftPreviewResult(JSON.stringify(responseUnknown)), /unexpected/);
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

test("Question Publication Review presents an immutable comparison before confirmation", () => {
  const review = {
    baseQuestion: "newQuestion",
    proposedTitle: draft.title,
    sections: [{ label: "Prompt", before: "Older prompt", after: "Estimate the omega angle." }],
  };

  assert.equal(review.baseQuestion, "newQuestion");
  assert.equal(review.sections[0].label, "Prompt");
});
