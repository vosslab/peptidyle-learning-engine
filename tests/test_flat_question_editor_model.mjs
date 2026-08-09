import assert from "node:assert/strict";
import test from "node:test";

import { createDefaultFlatQuestionSource } from "../src/features/flat_question_authoring/flat_question_defaults.ts";
import {
  addChoice,
  initialFlatQuestionEditorState,
  reduceFlatQuestionEditor,
  removeChoice,
  renameChoiceId,
  reorderChoices,
  setAttemptPolicy,
  setChoiceText,
  setCorrectChoice,
  setFlatQuestionTitle,
  setLanguage,
  setLicense,
  setOutcomeFeedback,
  setTags,
  setTaxonomy,
  setTimingPolicy,
  validateFlatQuestionSource,
} from "../src/features/flat_question_authoring/flat_question_editor_model.ts";

function source() {
  return createDefaultFlatQuestionSource();
}

test("editor only follows valid load, edit, save, and publish transitions", () => {
  const initial = initialFlatQuestionEditorState();
  assert.equal(reduceFlatQuestionEditor(initial, { kind: "saveStarted" }), initial);
  const loaded = reduceFlatQuestionEditor(initial, { kind: "loaded", source: source() });
  assert.equal(loaded.kind, "ready");
  assert.equal(loaded.status, "clean");
  const edited = setFlatQuestionTitle(source(), "Revised title");
  const dirty = reduceFlatQuestionEditor(loaded, { kind: "edit", source: edited });
  assert.equal(dirty.kind, "ready");
  assert.equal(dirty.status, "dirty");
  const saving = reduceFlatQuestionEditor(dirty, { kind: "saveStarted" });
  assert.equal(saving.kind, "ready");
  assert.equal(saving.status, "saving");
  const clean = reduceFlatQuestionEditor(saving, { kind: "saveSucceeded" });
  assert.equal(clean.kind, "ready");
  assert.equal(clean.status, "clean");
  const review = reduceFlatQuestionEditor(clean, { kind: "reviewOpened", review: "Ready" });
  assert.equal(review.kind, "publishReview");
  const publishing = reduceFlatQuestionEditor(review, { kind: "publishStarted" });
  assert.equal(publishing.kind, "publishing");
  const published = reduceFlatQuestionEditor(publishing, {
    kind: "publishSucceeded",
    reference: "v1",
  });
  assert.equal(published.kind, "published");
  assert.equal(reduceFlatQuestionEditor(published, { kind: "edit", source: source() }), published);
});

test("conflict and reload preserve local source while clearing protected preview", () => {
  const loaded = reduceFlatQuestionEditor(initialFlatQuestionEditorState(), {
    kind: "loaded",
    source: source(),
  });
  const previewed = reduceFlatQuestionEditor(loaded, {
    kind: "instructorPreviewLoaded",
    preview: { revision: '"1"', correctChoice: "choice_a", explanation: "Instructor only" },
  });
  const editedSource = setFlatQuestionTitle(source(), "Local only");
  const dirty = reduceFlatQuestionEditor(previewed, { kind: "edit", source: editedSource });
  assert.equal(dirty.kind, "ready");
  assert.equal(dirty.instructorPreview, null);
  const conflict = reduceFlatQuestionEditor(
    reduceFlatQuestionEditor(dirty, { kind: "saveStarted" }),
    { kind: "saveConflict" },
  );
  assert.equal(conflict.kind, "conflict");
  assert.equal(conflict.localSource.title, "Local only");
  const reloading = reduceFlatQuestionEditor(conflict, { kind: "reloadStarted" });
  const reloadFailed = reduceFlatQuestionEditor(reloading, {
    kind: "reloadFailed",
    message: "Network unavailable",
  });
  assert.equal(reloadFailed.kind, "conflict");
  assert.equal(reloadFailed.localSource.title, "Local only");
  const reloaded = reduceFlatQuestionEditor(reloading, {
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
  assert.equal(added.source.choices[2].id, "choice_1");
  const reordered = reorderChoices(added.source, ["choice_1", "choice_b", "choice_a"]);
  assert.deepEqual(
    reordered.source.choices.map((choice) => choice.id),
    ["choice_1", "choice_b", "choice_a"],
  );
  assert.equal(reordered.source.correctChoice, "choice_a");
  const removed = removeChoice(reordered.source, "choice_a");
  assert.equal(removed.source.correctChoice, "choice_b");
  assert.equal(removeChoice(source(), "choice_a").changed, false);
  assert.equal(renameChoiceId(source(), "choice_a", "Bad ID").changed, false);
  assert.equal(renameChoiceId(source(), "choice_a", "choice_b").changed, false);
  const renamed = renameChoiceId(source(), "choice_a", "correct_answer");
  assert.equal(renamed.source.correctChoice, "correct_answer");
  assert.equal(setCorrectChoice(source(), "missing").changed, false);
  assert.equal(setChoiceText(source(), "choice_a", "Edited").source.choices[0].text, "Edited");
});

test("policy and metadata helpers are immutable and validation gives safe author guidance", () => {
  const base = source();
  const edited = setLanguage(
    setLicense(
      setTaxonomy(
        setTags(
          setTimingPolicy(
            setAttemptPolicy(
              setOutcomeFeedback(base, { correct: "Good", incorrect: "Try again" }),
              { maxAttempts: 3, feedback: "deferred" },
            ),
            { kind: "perQuestion", seconds: 60, graceSeconds: 5 },
          ),
          ["biology", "assessment"],
        ),
        [{ scheme: "Bloom", code: "apply", label: "Apply" }],
      ),
      { kind: "ccBy" },
    ),
    "en",
  );
  assert.equal(base.language, "en-US");
  assert.deepEqual(edited.attemptPolicy, { maxAttempts: 3, feedback: "deferred" });
  assert.equal(edited.timingPolicy.kind, "perQuestion");
  assert.deepEqual(edited.tags, ["biology", "assessment"]);
  assert.equal(validateFlatQuestionSource(edited).valid, true);
  const invalid = setFlatQuestionTitle(edited, " ");
  const validation = validateFlatQuestionSource(invalid);
  assert.equal(validation.valid, false);
  assert.equal(validation.issues[0].field, "title");
  assert.equal(validation.issues[0].message.includes("Untitled question"), false);
  assert.equal(validation.issues[0].message.includes("Instructor"), false);
});
