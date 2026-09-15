import assert from "node:assert/strict";
import test from "node:test";

import {
  appendBlueprintCoursePage,
  appendPickedFixedEntries,
  appendPickedPool,
  blueprintCourseContinuationPresentation,
  blueprintLifecyclePresentation,
  emptyReusableContent,
  moveReusableEntry,
  validateBlueprintCourseContent,
  validateReusableContent,
} from "../src/features/blueprint_course/blueprint_course_model.ts";

function selection(...questionIds) {
  return { questionIds, questions: [] };
}

test("new Blueprint Assignment working state keeps answer-bearing feedback private", () => {
  const feedback = emptyReusableContent("Quiz").defaults.student_feedback_release_rule;

  assert.deepEqual(feedback, {
    score: "after_submit",
    per_item_correctness: "after_submit",
    submitted_response: "after_submit",
    question_feedback: "never",
    question_answer: "never",
    question_answer_explanation: "never",
    class_statistics: "never",
  });
});

test("Blueprint lifecycle choices expose only the server-directed state transitions", () => {
  const privateOwner = blueprintLifecyclePresentation("private", "blueprint_course_owner");
  assert.equal(privateOwner.canEdit, true);
  assert.equal(privateOwner.canPublish, true);
  assert.equal(privateOwner.canArchive, false);
  assert.equal(privateOwner.canAdopt, false);

  const publicReader = blueprintLifecyclePresentation("public", "active_instructor");
  assert.equal(publicReader.canAdopt, true);
  assert.equal(publicReader.canEdit, false);
  assert.equal(publicReader.canArchive, false);

  const publicOwner = blueprintLifecyclePresentation("public", "blueprint_course_owner");
  assert.equal(publicOwner.canArchive, true);
  assert.equal(publicOwner.canReturnToPrivate, true);

  const archivedOwner = blueprintLifecyclePresentation("archived", "blueprint_course_owner");
  assert.equal(archivedOwner.canRestore, true);
  assert.equal(archivedOwner.canAdopt, false);
  assert.equal(archivedOwner.canEdit, false);
});

test("new Blueprint Assignment working state uses the Assignment delivery defaults", () => {
  const defaults = emptyReusableContent("Quiz").defaults;

  assert.equal(defaults.late_work_rule, "reject");
  assert.equal(defaults.activity_rules.assignmentQuestionDisplayRule, "oneQuestionAtATime");
  assert.equal(defaults.activity_rules.assignmentQuestionOrderRule, "shuffled");
});

test("reusable entries preserve fixed and Question Pool interleaving", () => {
  const fixed = appendPickedFixedEntries(emptyReusableContent("Quiz"), selection("AAAA-ZBBB"));
  const pooled = appendPickedPool(fixed, selection("CCCD-XDDD", "EEEF-XFFF"));
  const reordered = moveReusableEntry(pooled, 1, -1);

  assert.deepEqual(
    reordered.entries.map((entry) => entry.kind),
    ["pool", "fixed"],
  );
  assert.equal(reordered.entries[0]?.kind === "pool" && reordered.entries[0].selection_count, 1);
});

test("Question Pool validation keeps selection count inside the selected Question Pool Item set", () => {
  const content = appendPickedPool(emptyReusableContent("Quiz"), selection("AAAA-ZBBB"));
  const pool = content.entries[0];
  const invalid =
    pool?.kind === "pool" ? { ...content, entries: [{ ...pool, selection_count: 2 }] } : content;

  assert.match(validateReusableContent(invalid).message ?? "", /selection count/);
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

test("Blueprint Course creation requires separate short and long lineage names", () => {
  const assignment = {
    ...emptyReusableContent("Ready assignment"),
    entries: [
      { kind: "fixed", question_id: "AAAA-ZBBB", points_possible: "1", scoring_rule: "normal" },
    ],
  };
  assert.equal(
    validateBlueprintCourseContent({
      short_name: "Blueprint",
      long_name: "Protein folding Blueprint Course",
      modules: [{ label: "Module 1", assignments: [assignment] }],
    }).valid,
    true,
  );
  assert.match(
    validateBlueprintCourseContent({
      short_name: " ",
      long_name: "Protein folding Blueprint Course",
      modules: [{ label: "Module 1", assignments: [assignment] }],
    }).message ?? "",
    /short and long names/,
  );
});
