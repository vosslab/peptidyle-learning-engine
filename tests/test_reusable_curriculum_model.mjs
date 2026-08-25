import assert from "node:assert/strict";
import test from "node:test";

import {
  alphaActionPresentation,
  alphaProblemPickerSources,
  appendCurriculumPage,
  appendPickedFixedEntries,
  appendPickedPool,
  curriculumContinuationPresentation,
  emptyAlphaDefinition,
  emptyReusableDefinition,
  moveAlphaModule,
  moveReusableEntry,
  updateReusableSchedule,
  validateReusableDefinition,
} from "../src/features/reusable_curriculum/reusable_curriculum_model.ts";

function selection(...questionIds) {
  return { questionIds, questions: [] };
}

test("reusable entries preserve fixed and pool interleaving", () => {
  const fixed = appendPickedFixedEntries(emptyReusableDefinition("Quiz"), selection("AAA-BBBB"));
  const pooled = appendPickedPool(fixed, selection("CCC-DDDD", "EEE-FFFF"));
  const reordered = moveReusableEntry(pooled, 1, -1);

  assert.deepEqual(
    reordered.entries.map((entry) => entry.kind),
    ["pool", "fixed"],
  );
});

test("pool validation keeps draw count inside the selected candidate set", () => {
  const definition = appendPickedPool(emptyReusableDefinition("Quiz"), selection("AAA-BBBB"));
  const pool = definition.entries[0];
  const invalid =
    pool?.kind === "pool" ? { ...definition, entries: [{ ...pool, drawCount: 2 }] } : definition;

  assert.match(validateReusableDefinition(invalid).message ?? "", /draw count/);
});

test("Alpha modules move without changing their reusable definitions", () => {
  const alpha = {
    ...emptyAlphaDefinition(),
    modules: [
      { label: "Cell biology", definitions: [emptyReusableDefinition("Membranes")] },
      { label: "Genetics", definitions: [emptyReusableDefinition("Recombination")] },
    ],
  };
  const moved = moveAlphaModule(alpha, 1, -1);

  assert.equal(moved.modules[0]?.definitions[0]?.title, "Recombination");
});

test("partial relative schedule accepts an independently useful due moment", () => {
  const scheduled = updateReusableSchedule(emptyReusableDefinition("Quiz"), "dueAt", {
    dayOffset: 7,
    localTime: "09:00:00.000",
  });

  assert.equal(
    validateReusableDefinition({
      ...scheduled,
      entries: [
        { kind: "fixed", questionId: "AAA-BBBB", pointsPossible: "1", scoringMode: "normal" },
      ],
    }).valid,
    true,
  );
});

test("creator and approved-reader access select the correct editing capability", () => {
  const creator = alphaActionPresentation("creator");
  const reader = alphaActionPresentation("approvedInstructor");

  assert.equal(creator.editable, true);
  assert.equal(reader.editable, false);
});

test("approved-reader access changes the next visible action", () => {
  const creator = alphaActionPresentation("creator");
  const reader = alphaActionPresentation("approvedInstructor");

  assert.notEqual(reader.primaryAction, creator.primaryAction);
});

test("Alpha authoring selects from the cross-tenant public library", () => {
  assert.deepEqual(alphaProblemPickerSources(), [
    { kind: "publicCatalog", label: "Public library" },
  ]);
});

test("load more keeps named visible curricula and appends the next live page", () => {
  const visible = appendCurriculumPage(
    [{ reference: "BP-one", title: "Enzyme kinetics" }],
    [
      { reference: "BP-one", title: "Stale duplicate" },
      { reference: "BP-two", title: "DNA repair" },
    ],
  );

  assert.deepEqual(
    visible.map((record) => record.title),
    ["Enzyme kinetics", "DNA repair"],
  );
});

test("continuation labels name the next action and disappear at the final page", () => {
  assert.deepEqual(curriculumContinuationPresentation("alpha", true, true), {
    visible: true,
    action: "Retry loading Alpha curricula",
  });
  assert.deepEqual(curriculumContinuationPresentation("blueprint", false, false), {
    visible: false,
    action: null,
  });
});
