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
  reusableContentInputFromView,
  validateBlueprintCourseContent,
  validateReusableContent,
} from "../src/features/blueprint_course/blueprint_course_model.ts";

function selection(...questionIds) {
  return {
    questionIds,
    questions: questionIds.map((questionId) => ({
      questionId,
      row: { questionRevisionTuple: { questionId, revisionNumber: 1 } },
    })),
  };
}

test("new Blueprint Assessment working state keeps answer-bearing feedback private", () => {
  const feedback =
    emptyReusableContent("regular_assignment").defaults.student_feedback_release_rule;

  assert.deepEqual(feedback, {
    score: "after_submit",
    per_item_correctness: "after_submit",
    submitted_response: "after_submit",
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
  assert.equal(publicOwner.canEdit, true);
  assert.equal(publicOwner.canArchive, true);
  assert.equal(publicOwner.canReturnToPrivate, true);

  const archivedOwner = blueprintLifecyclePresentation("archived", "blueprint_course_owner");
  assert.equal(archivedOwner.canRestore, true);
  assert.equal(archivedOwner.canAdopt, false);
  assert.equal(archivedOwner.canEdit, false);

  const archivedReader = blueprintLifecyclePresentation("archived", "active_instructor");
  assert.equal(archivedReader.canEdit, false);
});

test("new Blueprint Assessment working state uses the Assessment delivery defaults", () => {
  const defaults = emptyReusableContent("quiz", "Quiz").defaults;

  assert.equal(defaults.late_work_rule, "reject");
  assert.equal(defaults.assessment_attempt_limit, 1);
  assert.equal(defaults.student_feedback_release_rule.question_answer, "after_submit");
  assert.equal(defaults.student_feedback_release_rule.question_answer_explanation, "after_submit");
  assert.equal(defaults.activity_rules.assessmentQuestionOrderRule, "shuffled");
});

test("new regular Blueprint Assessment remains unlimited without a score or correctness gate", () => {
  const defaults = emptyReusableContent("regular_assignment").defaults;

  assert.equal(defaults.assessment_attempt_limit, null);
});

test("reusable entries preserve fixed and Question Pool interleaving", () => {
  const fixed = appendPickedFixedEntries(
    emptyReusableContent("quiz", "Quiz"),
    selection("AAAA-2BBB"),
  );
  const pooled = appendPickedPool(fixed, "CCCD-NDDD");
  const reordered = moveReusableEntry(pooled, 1, -1);

  assert.deepEqual(
    reordered.entries.map((entry) => entry.kind),
    ["pool", "fixed"],
  );
  assert.equal(reordered.entries[0]?.kind === "pool" && reordered.entries[0].selection_count, 1);
});

test("Question Pool validation requires a positive whole selection count", () => {
  const content = appendPickedPool(emptyReusableContent("quiz", "Quiz"), "AAAA-2BBB");
  const pool = content.entries[0];
  const invalid =
    pool?.kind === "pool" ? { ...content, entries: [{ ...pool, selection_count: 0 }] } : content;

  assert.match(validateReusableContent(invalid).message ?? "", /selection count/);
});

test("Blueprint Course pages append unique public IDs and name the next action", () => {
  const visible = appendBlueprintCoursePage(
    [{ id: "BP-one", title: "Enzyme kinetics" }],
    [
      { id: "BP-one", title: "Stale duplicate" },
      { id: "BP-two", title: "DNA repair" },
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
    ...emptyReusableContent("regular_assignment", "Ready assignment"),
    entries: [
      {
        kind: "fixed",
        question_revision_tuple: { questionId: "AAAA-2BBB", revisionNumber: 1 },
        points_possible: "1",
        scoring_rule: "normal",
      },
    ],
  };
  assert.equal(
    validateBlueprintCourseContent({
      short_name: "Blueprint",
      classification: {
        disciplineUuid: "018f5e7d-01b6-7c14-8a0b-4bfef6390d6d",
        subjectUuid: null,
        topicUuid: null,
        subtopicUuid: null,
        tags: [],
      },
      long_name: "Protein folding Blueprint Course",
      modules: [{ label: "Module 1", assessments: [assignment] }],
    }).valid,
    true,
  );
  assert.match(
    validateBlueprintCourseContent({
      short_name: " ",
      classification: {
        disciplineUuid: "018f5e7d-01b6-7c14-8a0b-4bfef6390d6d",
        subjectUuid: null,
        topicUuid: null,
        subtopicUuid: null,
        tags: [],
      },
      long_name: "Protein folding Blueprint Course",
      modules: [{ label: "Module 1", assessments: [assignment] }],
    }).message ?? "",
    /short and long names/,
  );
});

test("saved Blueprint Assessment mapping preserves its selected Type", () => {
  const defaults = emptyReusableContent("regular_assignment").defaults;
  const mapped = reusableContentInputFromView({
    assessment_type: "exam",
    title: "Final exam",
    instructions: "Show your reasoning.",
    entries: [],
    defaults,
  });

  assert.equal(mapped.assessment_type, "exam");
  assert.deepEqual(mapped.defaults, defaults);
});
