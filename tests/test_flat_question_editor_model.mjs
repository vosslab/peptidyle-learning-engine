import assert from "node:assert/strict";
import test from "node:test";

import { createDefaultFlatQuestionSource } from "../src/features/flat_question_authoring/flat_question_defaults.ts";
import {
  addMatchingPair,
  addChoice,
  initialFlatQuestionEditorState,
  reduceFlatQuestionEditor,
  removeChoice,
  removeMatchingPair,
  renameChoiceId,
  reorderChoices,
  reorderMatchingItems,
  setQuestionAttemptLimit,
  setChoiceText,
  setCorrectChoice,
  setFlatQuestionTitle,
  setLanguage,
  setMatchingItemText,
  setMatchingPair,
  setFlatQuestionResponseKind,
  setLicense,
  setOutcomeFeedback,
  setTags,
  setTaxonomy,
  setQuestionAttemptTimeLimit,
  validateFlatQuestionSource,
} from "../src/features/flat_question_authoring/flat_question_editor_model.ts";
import {
  addHotspotRegion,
  hotspotResponseFromAsset,
  hotspotSourceFromAsset,
  moveHotspotRegion,
  removeHotspotRegion,
  setHotspotCorrectRegion,
  setHotspotRegionCoordinate,
} from "../src/features/flat_question_authoring/flat_hotspot_editor_model.ts";

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
  assert.equal(renameChoiceId(source(), "choice_a", "Bad ID").changed, false);
  assert.equal(renameChoiceId(source(), "choice_a", "choice_b").changed, false);
  const renamed = renameChoiceId(source(), "choice_a", "correct_answer");
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
    setLicense(
      setTaxonomy(
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
        [{ scheme: "Bloom", code: "apply", label: "Apply" }],
      ),
      { kind: "ccBy" },
    ),
    "en",
  );
  assert.equal(base.language, "en-US");
  assert.deepEqual(edited.questionAttemptLimit, { maxAttempts: 3 });
  assert.equal(edited.questionAttemptTimeLimit.kind, "limited");
  assert.deepEqual(edited.tags, ["biology", "assessment"]);
  assert.equal(validateFlatQuestionSource(edited).valid, true);
  const invalid = setFlatQuestionTitle(edited, " ");
  const validation = validateFlatQuestionSource(invalid);
  assert.equal(validation.valid, false);
  assert.equal(validation.issues[0].field, "title");
  assert.equal(validation.issues[0].message.includes("Untitled question"), false);
  assert.equal(validation.issues[0].message.includes("Instructor"), false);
});

test("matching edits preserve semantic identities and prevent duplicate pair choices", () => {
  const matching = setFlatQuestionResponseKind(source(), "matching");
  const renamed = setMatchingItemText(matching, "prompts", "prompt_a", "Gene");
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
    const switched = setFlatQuestionResponseKind(source(), kind);
    assert.equal(switched.response.kind, kind);
    assert.equal(validateFlatQuestionSource(switched).valid, true);
  }
  const ordering = setFlatQuestionResponseKind(source(), "ordering");
  if (ordering.response.kind !== "ordering") throw new Error("Expected ordering source.");
  assert.deepEqual(ordering.response.correctOrder, ["item_a", "item_b", "item_c"]);
});

test("hotspot source starts only from a typed server descriptor and retains stable region identity", () => {
  const descriptor = {
    assetId: "aaaaaaaa-0000-4000-8000-000000000011",
    contentChecksum: "a".repeat(64),
    displayLabel: "Cell membrane diagram",
    mediaType: "image/png",
    intrinsicWidth: 800,
    intrinsicHeight: 600,
  };
  const response = hotspotResponseFromAsset(
    descriptor,
    "A membrane diagram with labeled features.",
  );
  assert.equal(response.surface.asset, descriptor.assetId);
  assert.equal(response.surface.checksum, descriptor.contentChecksum);
  assert.equal(response.regions[0]?.id, "region_1");
  const sourceWithHotspot = hotspotSourceFromAsset(
    source(),
    descriptor,
    response.surface.description,
  );
  assert.equal(validateFlatQuestionSource(sourceWithHotspot).valid, true);
  const added = addHotspotRegion(response);
  assert.equal(added.changed, true);
  const addedRegion = added.response.regions[1];
  assert.equal(addedRegion?.id, "region_2");
  const moved = moveHotspotRegion(added.response, "region_2", "earlier");
  assert.equal(moved.changed, true);
  assert.deepEqual(
    moved.response.regions.map((region) => region.id),
    ["region_2", "region_1"],
  );
  assert.equal(removeHotspotRegion(moved.response, "region_1").changed, false);
  assert.equal(setHotspotRegionCoordinate(response, "region_1", "width", 10_001).changed, false);
  assert.equal(setHotspotCorrectRegion(response, "region_1", false).changed, false);
});

test("matching pair operations retain semantic IDs and repair only the removed pair relation", () => {
  const matching = setFlatQuestionResponseKind(source(), "matching");
  const added = addMatchingPair(matching);
  assert.equal(added.changed, true);
  if (added.source.response.kind !== "matching") throw new Error("Expected matching source.");
  assert.deepEqual(added.source.response.matches.at(-1), {
    prompt: "prompt_1",
    choice: "choice_1",
  });
  const reordered = reorderMatchingItems(added.source, "prompts", [
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
