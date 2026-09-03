import assert from "node:assert/strict";
import test from "node:test";

import { createDefaultPleQuestionJsonSource } from "../src/features/ple_question_json_authoring/question_json_defaults.ts";
import {
  addMatchingPair,
  addChoice,
  initialPleQuestionJsonEditorState,
  reducePleQuestionJsonEditor,
  removeChoice,
  removeMatchingPair,
  renameQuestionChoiceReference,
  reorderChoices,
  reorderMatchingSide,
  setQuestionAttemptLimit,
  setChoiceText,
  setCorrectChoice,
  setPleQuestionJsonTitle,
  setLanguage,
  setMatchingSideText,
  setMatchingPair,
  setPleQuestionJsonResponseKind,
  setQuestionLicense,
  setQuestionHint,
  setOutcomeFeedback,
  setTags,
  setQuestionAttemptTimeLimit,
  validatePleQuestionJsonSource,
} from "../src/features/ple_question_json_authoring/question_json_editor_model.ts";

function source() {
  return createDefaultPleQuestionJsonSource();
}

test("editor only follows valid load, edit, save, and publish transitions", () => {
  const initial = initialPleQuestionJsonEditorState();
  assert.equal(reducePleQuestionJsonEditor(initial, { kind: "saveStarted" }), initial);
  const loaded = reducePleQuestionJsonEditor(initial, { kind: "loaded", source: source() });
  assert.equal(loaded.kind, "ready");
  assert.equal(loaded.status, "clean");
  const edited = setPleQuestionJsonTitle(source(), "Revised title");
  const dirty = reducePleQuestionJsonEditor(loaded, { kind: "edit", source: edited });
  assert.equal(dirty.kind, "ready");
  assert.equal(dirty.status, "dirty");
  const saving = reducePleQuestionJsonEditor(dirty, { kind: "saveStarted" });
  assert.equal(saving.kind, "ready");
  assert.equal(saving.status, "saving");
  const clean = reducePleQuestionJsonEditor(saving, { kind: "saveSucceeded" });
  assert.equal(clean.kind, "ready");
  assert.equal(clean.status, "clean");
  const review = reducePleQuestionJsonEditor(clean, { kind: "reviewOpened", review: "Ready" });
  assert.equal(review.kind, "publishReview");
  const publishing = reducePleQuestionJsonEditor(review, { kind: "publishStarted" });
  assert.equal(publishing.kind, "publishing");
  const published = reducePleQuestionJsonEditor(publishing, {
    kind: "publishSucceeded",
    reference: "v1",
  });
  assert.equal(published.kind, "published");
  assert.equal(
    reducePleQuestionJsonEditor(published, { kind: "edit", source: source() }),
    published,
  );
});

test("Question Hint editing remains separate from outcome feedback", () => {
  const withHint = setQuestionHint(source(), "Use the diagram labels before responding.");
  assert.equal(withHint.questionHint, "Use the diagram labels before responding.");
  assert.deepEqual(withHint.feedback, { correct: null, incorrect: null });
  assert.equal(setQuestionHint(withHint, null).questionHint, null);
});

test("conflict and reload preserve local source while clearing protected preview", () => {
  const loaded = reducePleQuestionJsonEditor(initialPleQuestionJsonEditorState(), {
    kind: "loaded",
    source: source(),
  });
  const previewed = reducePleQuestionJsonEditor(loaded, {
    kind: "instructorPreviewLoaded",
    preview: { revision: '"1"', correctChoice: "choice_a", explanation: "Instructor only" },
  });
  const editedSource = setPleQuestionJsonTitle(source(), "Local only");
  const dirty = reducePleQuestionJsonEditor(previewed, { kind: "edit", source: editedSource });
  assert.equal(dirty.kind, "ready");
  assert.equal(dirty.instructorPreview, null);
  const conflict = reducePleQuestionJsonEditor(
    reducePleQuestionJsonEditor(dirty, { kind: "saveStarted" }),
    { kind: "saveConflict" },
  );
  assert.equal(conflict.kind, "conflict");
  assert.equal(conflict.localSource.title, "Local only");
  const reloading = reducePleQuestionJsonEditor(conflict, { kind: "reloadStarted" });
  const reloadFailed = reducePleQuestionJsonEditor(reloading, {
    kind: "reloadFailed",
    message: "Network unavailable",
  });
  assert.equal(reloadFailed.kind, "conflict");
  assert.equal(reloadFailed.localSource.title, "Local only");
  const reloaded = reducePleQuestionJsonEditor(reloading, {
    kind: "reloadSucceeded",
    source: source(),
  });
  assert.equal(reloaded.kind, "ready");
  assert.equal(reloaded.instructorPreview, null);
});

test("choice edits retain semantic IDs and enforce choices and correct-answer invariants", () => {
  const initial = source();
  const added = addChoice(initial);
  assert.equal(added.changed, true);
  assert.equal(added.source.response.choices[2].id, "choice_1");
  const reordered = reorderChoices(added.source, ["choice_1", "choice_b", "choice_a"]);
  assert.deepEqual(
    reordered.source.response.choices.map((choice) => choice.id),
    ["choice_1", "choice_b", "choice_a"],
  );
  assert.equal(reordered.source.response.correctChoice, "choice_a");
  const removed = removeChoice(reordered.source, "choice_a");
  assert.equal(removed.source.response.correctChoice, "choice_b");
  assert.equal(removeChoice(source(), "choice_a").changed, false);
  assert.equal(renameQuestionChoiceReference(source(), "choice_a", "Bad ID").changed, false);
  assert.equal(renameQuestionChoiceReference(source(), "choice_a", "choice_b").changed, false);
  const renamed = renameQuestionChoiceReference(source(), "choice_a", "correct_answer");
  assert.equal(renamed.source.response.correctChoice, "correct_answer");
  assert.equal(setCorrectChoice(source(), "missing").changed, false);
  assert.equal(
    setChoiceText(source(), "choice_a", "Edited").source.response.choices[0].text,
    "Edited",
  );
});

test("policy and metadata helpers are immutable and validation gives safe author guidance", () => {
  const base = source();
  const edited = setLanguage(
    setQuestionLicense(
      setTags(
        setQuestionAttemptTimeLimit(
          setQuestionAttemptLimit(
            setOutcomeFeedback(base, { correct: "Good", incorrect: "Try again" }),
            { maxAttempts: 3 },
          ),
          { kind: "limited", seconds: 60, graceSeconds: 5 },
        ),
        ["biology", "assessment"],
      ),
      "CC-BY-4.0",
    ),
    "en",
  );
  assert.equal(base.language, "en-US");
  assert.deepEqual(edited.questionAttemptLimit, { maxAttempts: 3 });
  assert.equal(edited.questionAttemptTimeLimit.kind, "limited");
  assert.deepEqual(edited.tags, ["biology", "assessment"]);
  assert.equal(validatePleQuestionJsonSource(edited).valid, true);
  const invalid = setPleQuestionJsonTitle(edited, " ");
  const validation = validatePleQuestionJsonSource(invalid);
  assert.equal(validation.valid, false);
  assert.equal(validation.issues[0].field, "title");
  assert.equal(validation.issues[0].message.includes("Untitled question"), false);
  assert.equal(validation.issues[0].message.includes("Instructor"), false);
});

test("matching edits preserve semantic identities and prevent duplicate pair choices", () => {
  const matching = setPleQuestionJsonResponseKind(source(), "matching");
  const renamed = setMatchingSideText(matching, "prompts", "prompt_a", "Gene");
  assert.equal(renamed.source.response.kind, "matching");
  if (renamed.source.response.kind !== "matching") throw new Error("Expected matching source.");
  assert.equal(renamed.source.response.prompts[0]?.id, "prompt_a");
  const duplicate = setMatchingPair(renamed.source, "prompt_b", "choice_a");
  assert.equal(duplicate.changed, false);
});

test("ordinary Question Type switches use complete valid defaults with stable semantic IDs", () => {
  const expected = {
    multipleAnswer: true,
    fillIn: true,
    multiFillIn: true,
    numeric: true,
    ordering: true,
  };
  for (const kind of Object.keys(expected)) {
    const switched = setPleQuestionJsonResponseKind(source(), kind);
    assert.equal(switched.response.kind, kind);
    assert.equal(validatePleQuestionJsonSource(switched).valid, true);
  }
  const ordering = setPleQuestionJsonResponseKind(source(), "ordering");
  if (ordering.response.kind !== "ordering") throw new Error("Expected ordering source.");
  assert.deepEqual(ordering.response.correctOrder, ["item_a", "item_b", "item_c"]);
});

test("matching pair operations retain semantic IDs and repair only the removed pair relation", () => {
  const matching = setPleQuestionJsonResponseKind(source(), "matching");
  const added = addMatchingPair(matching);
  assert.equal(added.changed, true);
  if (added.source.response.kind !== "matching") throw new Error("Expected matching source.");
  assert.deepEqual(added.source.response.matches.at(-1), {
    prompt: "prompt_1",
    choice: "choice_1",
  });
  const reordered = reorderMatchingSide(added.source, "prompts", [
    "prompt_1",
    "prompt_a",
    "prompt_b",
  ]);
  assert.equal(reordered.changed, true);
  if (reordered.source.response.kind !== "matching") throw new Error("Expected matching source.");
  assert.deepEqual(reordered.source.response.matches.at(-1), {
    prompt: "prompt_1",
    choice: "choice_1",
  });
  const removed = removeMatchingPair(reordered.source, "prompt_1");
  if (removed.source.response.kind !== "matching") throw new Error("Expected matching source.");
  assert.equal(removed.changed, true);
  assert.equal(
    removed.source.response.prompts.some((item) => item.id === "prompt_1"),
    false,
  );
  assert.equal(
    removed.source.response.choices.some((item) => item.id === "choice_1"),
    false,
  );
  assert.equal(
    removed.source.response.matches.some((pair) => pair.prompt === "prompt_1"),
    false,
  );
});
