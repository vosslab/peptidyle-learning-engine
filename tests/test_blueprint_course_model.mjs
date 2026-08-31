import assert from "node:assert/strict";
import test from "node:test";

import {
  appendBlueprintCoursePage,
  appendPickedFixedEntries,
  appendPickedPool,
  blueprintCourseContinuationPresentation,
  emptyReusableDefinition,
  moveReusableEntry,
  updateReusableSchedule,
  validateReusableDefinition,
} from "../src/features/blueprint_course/blueprint_course_model.ts";

function selection(...questionIds) {
  return { questionIds, questions: [] };
}

test("reusable entries preserve fixed and Question Pool interleaving", () => {
  const fixed = appendPickedFixedEntries(emptyReusableDefinition("Quiz"), selection("AAA-BBBB"));
  const pooled = appendPickedPool(fixed, selection("CCC-DDDD", "EEE-FFFF"));
  const reordered = moveReusableEntry(pooled, 1, -1);

  assert.deepEqual(
    reordered.entries.map((entry) => entry.kind),
    ["pool", "fixed"],
  );
  assert.equal(reordered.entries[0]?.kind === "pool" && reordered.entries[0].draw_count, 1);
});

test("Question Pool validation keeps draw count inside the selected candidate set", () => {
  const definition = appendPickedPool(emptyReusableDefinition("Quiz"), selection("AAA-BBBB"));
  const pool = definition.entries[0];
  const invalid =
    pool?.kind === "pool" ? { ...definition, entries: [{ ...pool, draw_count: 2 }] } : definition;

  assert.match(validateReusableDefinition(invalid).message ?? "", /draw count/);
});

test("relative schedule keeps a valid independently useful due moment", () => {
  const scheduled = updateReusableSchedule(emptyReusableDefinition("Quiz"), "due_at", {
    day_offset: 7,
    local_time: "09:00:00.000",
  });

  assert.equal(
    validateReusableDefinition({
      ...scheduled,
      entries: [
        { kind: "fixed", question_id: "AAA-BBBB", points_possible: "1", scoring_rule: "normal" },
      ],
    }).valid,
    true,
  );
});

test("Blueprint Course pages append unique public references and name the next action", () => {
  const visible = appendBlueprintCoursePage(
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
  assert.deepEqual(blueprintCourseContinuationPresentation(true, true), {
    visible: true,
    action: "Retry loading Blueprint Courses",
  });
  assert.deepEqual(blueprintCourseContinuationPresentation(false, false), {
    visible: false,
    action: null,
  });
});
